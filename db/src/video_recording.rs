use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};
use uuid;

use printnanny_api_client::models;
use printnanny_os_models;

use crate::connection::establish_sqlite_connection;
use crate::schema::video_recording_parts;
use crate::schema::video_recordings;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub cloud_sync_done: bool,
    pub dir: String,
    pub finalize_start: Option<DateTime<Utc>>,
    pub finalize_end: Option<DateTime<Utc>>,
    pub recording_start: Option<DateTime<Utc>>,
    pub recording_end: Option<DateTime<Utc>>,
    pub gcode_file_name: Option<String>,
}

// sqlite does not support unsigned integers, so we need to cast to/from u32 and u64
#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[diesel(table_name = video_recording_parts)]
pub struct VideoRecordingPart {
    pub id: String,
    pub size: i64,
    pub buffer_index: i64,
    pub buffer_runningtime: i64,
    pub deleted: bool,
    pub sync_start: Option<DateTime<Utc>>,
    pub sync_end: Option<DateTime<Utc>>,
    pub file_name: String,
    pub video_recording_id: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = video_recordings)]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub cloud_sync_done: &'a bool,
    pub dir: &'a str,
}

// sqlite does not support unsigned integers, so we need to cast to/from u32 and u64
#[derive(Debug, Insertable)]
#[diesel(table_name = video_recording_parts)]
pub struct NewVideoRecordingPart<'a> {
    pub id: &'a str,
    pub size: &'a i64,
    pub buffer_index: &'a i64,
    pub buffer_runningtime: &'a i64,
    pub deleted: &'a bool,
    pub file_name: &'a str,
    pub video_recording_id: &'a str,
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = video_recordings)]
pub struct UpdateVideoRecording<'a> {
    pub cloud_sync_done: Option<&'a bool>,
    pub dir: Option<&'a str>,
    pub recording_start: Option<&'a DateTime<Utc>>,
    pub recording_end: Option<&'a DateTime<Utc>>,
    pub gcode_file_name: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = video_recording_parts)]
pub struct UpdateVideoRecordingPart<'a> {
    pub deleted: Option<&'a bool>,
    pub sync_start: Option<&'a DateTime<Utc>>,
    pub sync_end: Option<&'a DateTime<Utc>>,
}

impl VideoRecording {
    pub fn update_from_cloud(
        connection_str: &str,
        obj: &models::VideoRecording,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);

        let r_start_value = obj.recording_start.as_ref().map(|v| {
            <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<DateTime<Utc>>>::into(
                DateTime::parse_from_rfc3339(v).unwrap(),
            )
        });
        let r_end_value = obj.recording_end.as_ref().map(|v| {
            <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<DateTime<Utc>>>::into(
                DateTime::parse_from_rfc3339(v).unwrap(),
            )
        });

        let row = UpdateVideoRecording {
            recording_end: r_end_value.as_ref(),
            recording_start: r_start_value.as_ref(),
            gcode_file_name: None,
            dir: None,
            cloud_sync_done: obj.cloud_sync_done.as_ref(),
        };

        diesel::update(video_recordings.filter(id.eq(&obj.id.clone().unwrap())))
            .set(row)
            .execute(connection)?;

        Ok(())
    }

    pub fn update(
        connection_str: &str,
        row_id: &str,
        row: UpdateVideoRecording,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        diesel::update(video_recordings.filter(id.eq(row_id)))
            .set(row)
            .execute(connection)?;
        info!("Updated VideoRecording with id {}", row_id);
        Ok(())
    }
    pub fn get_by_id(
        connection_str: &str,
        row_id: &str,
    ) -> Result<VideoRecording, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        video_recordings
            .filter(id.eq(row_id))
            .first::<VideoRecording>(connection)
    }
    pub fn get_all(connection_str: &str) -> Result<Vec<VideoRecording>, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result = video_recordings
            .order_by(id)
            .load::<VideoRecording>(connection)?;
        Ok(result)
    }
    pub fn get_current(
        connection_str: &str,
    ) -> Result<Option<VideoRecording>, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result = video_recordings
            .filter(recording_end.is_null())
            .order(recording_start.desc())
            .first::<VideoRecording>(connection)
            .optional()?;
        Ok(result)
    }

    pub fn finish_all(connection_str: &str) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);

        // select all unfinished recordings
        let unfinished_recordings = video_recordings
            .filter(recording_end.is_null())
            .load::<VideoRecording>(connection)?;
        let count = unfinished_recordings.len();

        if count > 1 {
            let now = Utc::now();
            info!(
                "Marking {} VideoRecording rows as finished",
                unfinished_recordings.len()
            );
            let row = UpdateVideoRecording {
                recording_end: Some(&now),
                cloud_sync_done: None,
                dir: None,
                recording_start: None,
                gcode_file_name: None,
            };
            diesel::update(video_recordings.filter(recording_end.is_null()))
                .set(row)
                .execute(connection)?;
        } else {
            info!("No unfinished VideoRecordings found");
        }

        Ok(())
    }

    pub fn start_new(
        connection_str: &str,
        video_path: PathBuf,
    ) -> Result<VideoRecording, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);

        let row_id = uuid::Uuid::new_v4().to_string();
        let dirname = video_path.join(&row_id);
        fs::create_dir_all(&dirname).expect(&format!(
            "Failed to create directory {}",
            &dirname.display()
        ));
        info!("Created {}", dirname.display());
        let row = NewVideoRecording {
            id: &row_id,
            cloud_sync_done: &false,
            dir: &dirname.display().to_string(),
        };
        diesel::insert_into(video_recordings)
            .values(&row)
            .execute(connection)?;
        info!("Created new VideoRecording with id {}", &row_id);
        let result = video_recordings.find(&row_id).first(connection)?;
        Ok(result)
    }
}

impl From<VideoRecording> for printnanny_os_models::VideoRecording {
    fn from(obj: VideoRecording) -> Self {
        Self {
            id: obj.id,
            dir: obj.dir,
            cloud_sync_done: obj.cloud_sync_done,
            recording_start: obj.recording_start.map(|v| v.to_rfc3339()),
            recording_end: obj.recording_end.map(|v| v.to_rfc3339()),
            gcode_file_name: obj.gcode_file_name,
        }
    }
}

impl From<VideoRecording> for models::VideoRecordingRequest {
    fn from(obj: VideoRecording) -> Self {
        Self {
            id: Some(obj.id),
            finalize_start: obj.finalize_start.map(|v| v.to_rfc3339()),
            finalize_end: obj.finalize_end.map(|v| v.to_rfc3339()),
            cloud_sync_done: Some(obj.cloud_sync_done),
            recording_start: obj.recording_start.map(|v| v.to_rfc3339()),
            recording_end: obj.recording_end.map(|v| v.to_rfc3339()),
            gcode_file_name: obj.gcode_file_name,
        }
    }
}

// parse recording id from path like: /home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4
pub fn parse_video_recording_id(filename: &str) -> String {
    let path = PathBuf::from(filename);
    let mut components = path.components();
    components
        .nth_back(1)
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap()
        .into()
}

pub fn parse_video_recording_index(filename: &str) -> i64 {
    let path = PathBuf::from(filename);
    let mut components = path.components();
    let last: String = components
        .nth_back(0)
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap()
        .into();
    last.split('.').next().unwrap().parse().unwrap()
}

impl VideoRecordingPart {
    pub fn row_id_from_filename(filename: &str) -> String {
        let video_recording_id = parse_video_recording_id(filename);
        let index = parse_video_recording_index(filename);
        format!("{video_recording_id}__{index}")
    }

    pub fn insert(
        connection_str: &str,
        row: NewVideoRecordingPart,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        diesel::insert_into(video_recording_parts)
            .values(&row)
            .execute(connection)?;
        Ok(())
    }

    pub fn get_by_id(
        connection_str: &str,
        row_id: &str,
    ) -> Result<VideoRecordingPart, diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result = video_recording_parts.find(&row_id).first(connection)?;
        Ok(result)
    }

    pub fn update_from_cloud(
        connection_str: &str,
        obj: &models::VideoRecordingPart,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);

        let sync_start_value =
            <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<DateTime<Utc>>>::into(
                DateTime::parse_from_rfc3339(&obj.sync_start).unwrap(),
            );
        let sync_end_value = <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<
            DateTime<Utc>,
        >>::into(DateTime::parse_from_rfc3339(&obj.sync_end).unwrap());

        let row_update = UpdateVideoRecordingPart {
            deleted: None,
            sync_start: Some(&sync_start_value),
            sync_end: Some(&sync_end_value),
        };
        diesel::update(video_recording_parts.filter(id.eq(&obj.id)))
            .set(row_update)
            .execute(connection)?;
        Ok(())
    }

    pub fn get_ready_for_cloud_sync(
        connection_str: &str,
    ) -> Result<Vec<VideoRecordingPart>, diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result = video_recording_parts
            .filter(sync_start.is_null())
            .load::<VideoRecordingPart>(connection)?;
        Ok(result)
    }

    pub fn get_parts_by_video_recording_id(
        connection_str: &str,
        video_recording: &str,
    ) -> Result<Vec<VideoRecordingPart>, diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        let result = video_recording_parts
            .filter(video_recording_id.eq(video_recording))
            .load::<VideoRecordingPart>(connection)?;
        Ok(result)
    }

    pub fn update(
        connection_str: &str,
        row_id: &str,
        row: UpdateVideoRecordingPart,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);
        diesel::update(video_recording_parts.filter(id.eq(row_id)))
            .set(row)
            .execute(connection)?;
        info!("Updated VideoRecordingPart with id {}", row_id);
        Ok(())
    }
}

impl From<VideoRecordingPart> for printnanny_os_models::VideoRecordingPart {
    fn from(obj: VideoRecordingPart) -> Self {
        Self {
            id: obj.id,
            deleted: obj.deleted,
            size: obj.size,
            buffer_index: obj.buffer_index,
            buffer_runningtime: obj.buffer_runningtime,
            video_recording_id: obj.video_recording_id,
            sync_start: obj.sync_start.map(|v| v.to_rfc3339()),
            sync_end: obj.sync_end.map(|v| v.to_rfc3339()),
            file_name: obj.file_name,
        }
    }
}

impl From<&printnanny_os_models::VideoRecordingPart> for VideoRecordingPart {
    fn from(obj: &printnanny_os_models::VideoRecordingPart) -> Self {
        let sync_start: Option<DateTime<Utc>> = obj
            .sync_start
            .as_ref()
            .map(|v| DateTime::parse_from_rfc3339(v).unwrap().into());
        let sync_end: Option<DateTime<Utc>> = obj
            .sync_end
            .as_ref()
            .map(|v| DateTime::parse_from_rfc3339(v).unwrap().into());
        Self {
            id: obj.id.clone(),
            deleted: obj.deleted,
            size: obj.size,
            buffer_index: obj.buffer_index,
            buffer_runningtime: obj.buffer_runningtime,
            video_recording_id: obj.video_recording_id.clone(),
            file_name: obj.file_name.clone(),
            sync_start,
            sync_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_video_recording_id() {
        let filename = "/home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4";
        let expected = "66b3a3a0-30b5-41f2-9907-a335de57c921";
        let result = parse_video_recording_id(filename);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_video_recording_index() {
        let filename = "/home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4";
        let expected = 25;
        let result = parse_video_recording_index(filename);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_row_id_format() {
        let filename = "/home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4";
        let result = VideoRecordingPart::row_id_from_filename(filename);
        let expected = "66b3a3a0-30b5-41f2-9907-a335de57c921__25";

        assert_eq!(result, expected);
    }
}

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use log::info;
use uuid;

use printnanny_api_client::models;
use printnanny_asyncapi_models;

use crate::connection::establish_sqlite_connection;
use crate::schema::video_recording_parts;
use crate::schema::video_recordings;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub cloud_sync_done: bool,
    pub dir: String,
    pub recording_start: Option<DateTime<Utc>>,
    pub recording_end: Option<DateTime<Utc>>,
    pub gcode_file_name: Option<String>,
}

// sqlite does not support unsigned integers, so we need to cast to/from u32 and u64
#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = video_recording_parts)]
pub struct VideoRecordingPart {
    pub id: String,
    pub size: i64,
    pub buffer_index: i32,
    pub buffer_ts: i64,
    pub buffer_streamtime: i64,
    pub buffer_runningtime: i64,
    pub buffer_duration: i64,
    pub buffer_offset: i64,
    pub buffer_offset_end: i64,
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
    pub buffer_index: &'a i32,
    pub buffer_ts: &'a i64,
    pub buffer_streamtime: &'a i64,
    pub buffer_runningtime: &'a i64,
    pub buffer_duration: &'a i64,
    pub buffer_offset: &'a i64,
    pub buffer_offset_end: &'a i64,
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

    // pub fn get_ready_for_cloud_sync(
    //     connection_str: &str,
    // ) -> Result<Vec<VideoRecording>, diesel::result::Error> {
    //     use crate::schema::video_recordings::dsl::*;
    //     let connection = &mut establish_sqlite_connection(connection_str);
    //     let result = video_recordings
    //         .filter(
    //             recording_status
    //                 .eq("done")
    //                 .and(cloud_sync_status.eq("pending"))
    //                 .and(cloud_sync_start.is_null()),
    //         )
    //         .load::<VideoRecording>(connection)?;

    //     info!("VideoRecording rows ready for cloud sync: {:#?}", &result);
    //     Ok(result)
    // }

    // pub fn start_cloud_sync(
    //     connection_str: &str,
    //     row_id: &str,
    // ) -> Result<(), diesel::result::Error> {
    //     let now = Utc::now();
    //     let row = UpdateVideoRecording {
    //         cloud_sync_start: Some(&now),
    //         cloud_sync_status: Some("inprogress"),
    //         deleted: None,
    //         cloud_sync_percent: None,
    //         gcode_file_name: None,
    //         recording_status: None,
    //         recording_start: None,
    //         recording_end: None,
    //         mp4_upload_url: None,
    //         mp4_download_url: None,
    //         cloud_sync_end: None,
    //     };
    //     Self::update(connection_str, row_id, row)
    // }

    // pub fn set_cloud_sync_progress(
    //     connection_str: &str,
    //     row_id: &str,
    //     progress: &i32,
    // ) -> Result<(), diesel::result::Error> {
    //     let row = UpdateVideoRecording {
    //         cloud_sync_percent: Some(progress),
    //         deleted: None,
    //         gcode_file_name: None,
    //         recording_status: None,
    //         recording_start: None,
    //         recording_end: None,
    //         mp4_upload_url: None,
    //         mp4_download_url: None,
    //         cloud_sync_status: None,
    //         cloud_sync_start: None,
    //         cloud_sync_end: None,
    //     };
    //     Self::update(connection_str, row_id, row)
    // }

    // pub fn finish_cloud_sync(
    //     connection_str: &str,
    //     row_id: &str,
    // ) -> Result<(), diesel::result::Error> {
    //     let now = Utc::now();
    //     let row = UpdateVideoRecording {
    //         cloud_sync_percent: Some(&100),
    //         cloud_sync_end: Some(&now),
    //         cloud_sync_status: None,
    //         deleted: None,
    //         gcode_file_name: None,
    //         recording_status: None,
    //         recording_start: None,
    //         recording_end: None,
    //         mp4_upload_url: None,
    //         mp4_download_url: None,
    //         cloud_sync_start: None,
    //     };
    //     Self::update(connection_str, row_id, row)
    // }

    // pub fn stop_all(connection: &str) -> Result<(), diesel::result::Error> {
    //     use crate::schema::video_recordings::dsl::*;
    //     let connection = &mut establish_sqlite_connection(connection);

    //     diesel::update(video_recordings)
    //         .set(recording_status.eq("done"))
    //         .execute(connection)?;
    //     info!("Set existing VideoRecording.recording_status = done");
    //     Ok(())
    // }

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

impl From<VideoRecording> for printnanny_asyncapi_models::VideoRecording {
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
            cloud_sync_done: Some(obj.cloud_sync_done),
            combine_done: Some(false),
            recording_start: obj.recording_start.map(|v| v.to_rfc3339()),
            recording_end: obj.recording_end.map(|v| v.to_rfc3339()),
            gcode_file_name: obj.gcode_file_name,
        }
    }
}

impl VideoRecordingPart {
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

    pub fn update_from_cloud(
        connection_str: &str,
        obj: &models::VideoRecordingPart,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recording_parts::dsl::*;
        let connection = &mut establish_sqlite_connection(connection_str);

        let sync_start_value = obj.sync_start.as_ref().map(|v| {
            <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<DateTime<Utc>>>::into(
                DateTime::parse_from_rfc3339(v).unwrap(),
            )
        });
        let sync_end_value = obj.sync_end.as_ref().map(|v| {
            <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<DateTime<Utc>>>::into(
                DateTime::parse_from_rfc3339(v).unwrap(),
            )
        });
        let row_update = UpdateVideoRecordingPart {
            deleted: None,
            sync_start: sync_start_value.as_ref(),
            sync_end: sync_end_value.as_ref(),
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
}

impl From<VideoRecordingPart> for models::VideoRecordingPartRequest {
    fn from(obj: VideoRecordingPart) -> Self {
        Self {
            id: obj.id,
            size: obj.size,
            buffer_index: obj.buffer_index as i64,
            buffer_duration: obj.buffer_duration,
            buffer_ts: obj.buffer_ts,
            buffer_runningtime: obj.buffer_runningtime,
            buffer_streamtime: obj.buffer_streamtime,
            buffer_offset: obj.buffer_offset,
            buffer_offset_end: obj.buffer_offset_end,
            sync_start: None,
            sync_end: None,
            video_recording: obj.video_recording_id,
            file_name: obj.file_name,
        }
    }
}

impl From<VideoRecordingPart> for printnanny_asyncapi_models::VideoRecordingPart {
    fn from(obj: VideoRecordingPart) -> Self {
        Self {
            id: obj.id,
            deleted: obj.deleted,
            size: obj.size,
            buffer_duration: obj.buffer_duration,
            buffer_index: obj.buffer_index,
            buffer_ts: obj.buffer_ts,
            buffer_runningtime: obj.buffer_runningtime,
            buffer_streamtime: obj.buffer_streamtime,
            buffer_offset: obj.buffer_offset,
            buffer_offset_end: obj.buffer_offset_end,

            video_recording_id: obj.video_recording_id,
            sync_start: obj.sync_start.map(|v| v.to_rfc3339()),
            sync_end: obj.sync_end.map(|v| v.to_rfc3339()),
            file_name: obj.file_name,
        }
    }
}

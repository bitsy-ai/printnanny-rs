use chrono::{DateTime, Utc};
use diesel::prelude::*;
use log::info;
use printnanny_settings::printnanny::PrintNannySettings;
use uuid;

use printnanny_api_client::models;
use printnanny_asyncapi_models;

use crate::connection::establish_sqlite_connection;
use crate::schema::video_recordings;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub deleted: bool,
    pub recording_status: String,
    pub recording_start: Option<DateTime<Utc>>,
    pub recording_end: Option<DateTime<Utc>>,
    pub mp4_file_name: String,
    pub mp4_upload_url: Option<String>,
    pub mp4_download_url: Option<String>,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: String,
    pub cloud_sync_percent: Option<i32>,
    pub cloud_sync_start: Option<DateTime<Utc>>,
    pub cloud_sync_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = video_recordings)]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub recording_status: &'a str,
    pub cloud_sync_status: &'a str,
    pub mp4_file_name: &'a str,
    pub gcode_file_name: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = video_recordings)]
pub struct UpdateVideoRecording<'a> {
    pub deleted: Option<&'a bool>,
    pub gcode_file_name: Option<&'a str>,
    pub recording_status: Option<&'a str>,
    pub recording_start: Option<&'a DateTime<Utc>>,
    pub recording_end: Option<&'a DateTime<Utc>>,
    pub mp4_upload_url: Option<&'a str>,
    pub mp4_download_url: Option<&'a str>,
    pub cloud_sync_status: Option<&'a str>,
    pub cloud_sync_percent: Option<&'a i32>,
    pub cloud_sync_start: Option<&'a DateTime<Utc>>,
    pub cloud_sync_end: Option<&'a DateTime<Utc>>,
}

impl VideoRecording {
    pub fn to_openapi_recording_status(status: &str) -> Option<models::RecordingStatusEnum> {
        match status {
            "done" => Some(models::RecordingStatusEnum::Done),
            "progress" => Some(models::RecordingStatusEnum::Inprogress),
            "pending" => Some(models::RecordingStatusEnum::Pending),
            _ => None,
        }
    }
    pub fn to_asyncapi_recording_status(
        status: &str,
    ) -> Option<printnanny_asyncapi_models::VideoRecordingStatus> {
        match status {
            "done" => Some(printnanny_asyncapi_models::VideoRecordingStatus::Done),
            "progress" => Some(printnanny_asyncapi_models::VideoRecordingStatus::Inprogress),
            "pending" => Some(printnanny_asyncapi_models::VideoRecordingStatus::Pending),
            _ => None,
        }
    }
    pub fn update(row_id: &str, row: UpdateVideoRecording) -> Result<(), diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        diesel::update(video_recordings.filter(id.eq(row_id)))
            .set(row)
            .execute(connection)?;
        info!("Updated VideoRecording with id {}", row_id);
        Ok(())
    }
    pub fn get_by_id(row_id: &str) -> Result<VideoRecording, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        video_recordings
            .filter(id.eq(row_id))
            .first::<VideoRecording>(connection)
    }
    pub fn get_all() -> Result<Vec<VideoRecording>, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        let result = video_recordings
            .order_by(id)
            .load::<VideoRecording>(connection)?;
        Ok(result)
    }
    pub fn get_current() -> Result<Option<VideoRecording>, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        let result = video_recordings
            .filter(recording_status.eq("inprogress"))
            .order(recording_start.desc())
            .first::<VideoRecording>(connection)
            .optional()?;
        Ok(result)
    }

    pub fn get_ready_for_cloud_sync() -> Result<Vec<VideoRecording>, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        let result = video_recordings
            .filter(
                recording_status
                    .eq("done")
                    .and(cloud_sync_status.eq("pending"))
                    .and(cloud_sync_start.is_null()),
            )
            .load::<VideoRecording>(connection)?;

        info!("VideoRecording rows ready for cloud sync: {:#?}", &result);
        Ok(result)
    }

    pub fn start_cloud_sync(row_id: &str) -> Result<(), diesel::result::Error> {
        let now = Utc::now();
        let row = UpdateVideoRecording {
            cloud_sync_start: Some(&now),
            cloud_sync_status: Some("inprogress"),
            deleted: None,
            cloud_sync_percent: None,
            gcode_file_name: None,
            recording_status: None,
            recording_start: None,
            recording_end: None,
            mp4_upload_url: None,
            mp4_download_url: None,
            cloud_sync_end: None,
        };
        Self::update(row_id, row)
    }

    pub fn set_cloud_sync_progress(
        row_id: &str,
        progress: &i32,
    ) -> Result<(), diesel::result::Error> {
        let row = UpdateVideoRecording {
            cloud_sync_percent: Some(progress),
            deleted: None,
            gcode_file_name: None,
            recording_status: None,
            recording_start: None,
            recording_end: None,
            mp4_upload_url: None,
            mp4_download_url: None,
            cloud_sync_status: None,
            cloud_sync_start: None,
            cloud_sync_end: None,
        };
        Self::update(row_id, row)
    }

    pub fn finish_cloud_sync(row_id: &str) -> Result<(), diesel::result::Error> {
        let now = Utc::now();
        let row = UpdateVideoRecording {
            cloud_sync_percent: Some(&100),
            cloud_sync_end: Some(&now),
            cloud_sync_status: None,
            deleted: None,
            gcode_file_name: None,
            recording_status: None,
            recording_start: None,
            recording_end: None,
            mp4_upload_url: None,
            mp4_download_url: None,
            cloud_sync_start: None,
        };
        Self::update(row_id, row)
    }

    pub fn stop_all() -> Result<(), diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();

        diesel::update(video_recordings)
            .set(recording_status.eq("done"))
            .execute(connection)?;
        info!("Set existing VideoRecording.recording_status = done");
        Ok(())
    }
    pub fn start_new() -> Result<VideoRecording, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        let connection = &mut establish_sqlite_connection();
        // mark all other recordings as done
        Self::stop_all()?;
        let settings = PrintNannySettings::new().unwrap();
        let row_id = uuid::Uuid::new_v4().to_string();
        let filename = settings.paths.video().join(format!("{}.mp4", &row_id));
        let row = NewVideoRecording {
            id: &row_id,
            recording_status: "pending",
            cloud_sync_status: "pending",
            mp4_file_name: &filename.display().to_string(),
            gcode_file_name: None, // TODO
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
        let recording_status =
            VideoRecording::to_asyncapi_recording_status(&obj.recording_status).map(Box::new);
        let cloud_sync_status =
            VideoRecording::to_asyncapi_recording_status(&obj.cloud_sync_status).map(Box::new);

        Self {
            id: obj.id,
            recording_status,
            recording_start: obj.recording_start.map(|v| v.to_rfc3339()),
            recording_end: obj.recording_end.map(|v| v.to_rfc3339()),
            mp4_file_name: obj.mp4_file_name,
            mp4_upload_url: obj.mp4_upload_url,
            mp4_download_url: obj.mp4_download_url,
            gcode_file_name: obj.gcode_file_name,
            cloud_sync_status,
            cloud_sync_start: obj.cloud_sync_start.map(|v| v.to_rfc3339()),
            cloud_sync_end: obj.cloud_sync_end.map(|v| v.to_rfc3339()),
        }
    }
}

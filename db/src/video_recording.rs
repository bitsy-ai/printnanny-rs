use chrono::{DateTime, Utc};
use diesel::prelude::*;
use log::info;
use printnanny_settings::printnanny::PrintNannySettings;
use uuid;

use printnanny_asyncapi_models;

use crate::connection::establish_sqlite_connection;
use crate::schema::video_recordings;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub recording_status: String,
    pub recording_start: Option<DateTime<Utc>>,
    pub recording_end: Option<DateTime<Utc>>,
    pub mp4_file_name: String,
    pub mp4_upload_url: Option<String>,
    pub mp4_download_url: Option<String>,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: String,
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
    pub gcode_file_name: Option<&'a str>,
    pub recording_status: Option<&'a str>,
    pub recording_start: Option<&'a DateTime<Utc>>,
    pub recording_end: Option<&'a DateTime<Utc>>,
    pub mp4_upload_url: Option<&'a str>,
    pub mp4_download_url: Option<&'a str>,
    pub cloud_sync_status: Option<&'a str>,
    pub cloud_sync_start: Option<DateTime<Utc>>,
    pub cloud_sync_end: Option<DateTime<Utc>>,
}

impl VideoRecording {
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
    pub fn start_new() -> Result<VideoRecording, diesel::result::Error> {
        use crate::schema::video_recordings::dsl::*;
        // mark all other recordings as done
        let connection = &mut establish_sqlite_connection();

        diesel::update(video_recordings)
            .set(recording_status.eq("done"))
            .execute(connection)?;
        info!("Set existing VideoRecording.recording_status = done");
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
        Self {
            id: obj.id,
            recording_status: Box::new(
                serde_json::from_str::<printnanny_asyncapi_models::VideoRecordingStatus>(
                    &obj.recording_status,
                )
                .unwrap(),
            ),
            recording_start: obj.recording_start.map(|v| v.to_rfc3339()),
            recording_end: obj.recording_end.map(|v| v.to_rfc3339()),
            mp4_file_name: obj.mp4_file_name,
            mp4_upload_url: obj.mp4_upload_url,
            mp4_download_url: obj.mp4_download_url,
            gcode_file_name: obj.gcode_file_name,
            cloud_sync_status: Box::new(
                serde_json::from_str::<printnanny_asyncapi_models::VideoRecordingStatus>(
                    &obj.cloud_sync_status,
                )
                .unwrap(),
            ),
            cloud_sync_start: obj.cloud_sync_start.map(|v| v.to_rfc3339()),
            cloud_sync_end: obj.cloud_sync_end.map(|v| v.to_rfc3339()),
        }
    }
}

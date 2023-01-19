use chrono::{DateTime, Utc};
use diesel::prelude::*;

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
}

// impl From<String> for printnanny_asyncapi_models::VideoRecordingStatus {
//     fn from(value: String) -> Self {
//         match &value {
//             "pending" => printnanny_asyncapi_models::VideoRecordingStatus::Pending,
//             "inprogress" => printnanny_asyncapi_models::VideoRecordingStatus::Inprogress,
//             "done" => printnanny_asyncapi_models::VideoRecordingStatus::Done,
//             _ => panic!(
//                 "Invalid value for printnanny_asyncapi_models::VideoRecordingStatus: {}",
//                 &value
//             ),
//         }
//     }
// }

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

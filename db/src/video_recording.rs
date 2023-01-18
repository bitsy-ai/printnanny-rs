use diesel::prelude::*;
use diesel::sql_types::SqlType;
use serde::{Deserialize, Serialize};

use crate::schema::video_recordings;
use crate::sql_types::RecordingStatus;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub recording_status: RecordingStatus,
    pub recording_start: Option<u64>,
    pub recording_end: Option<u64>,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: RecordingStatus,
    pub cloud_sync_start: Option<u64>,
    pub cloud_sync_end: Option<u64>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = video_recordings)]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub recording_file_name: &'a str,
    pub gcode_file_name: &'a str,
}

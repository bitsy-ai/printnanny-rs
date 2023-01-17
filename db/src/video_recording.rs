use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::enums::Status;
use crate::schema::video_recordings;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub recording_status: Status,
    pub recording_start: Option<u64>,
    pub recording_end: Option<u64>,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: Status,
    pub cloud_sync_start: Option<u64>,
    pub cloud_sync_end: Option<u64>,
}

#[derive(Insertable)]
#[diesel(table_name = video_recordings)]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub recording_file_name: &'a str,
    pub gcode_file_name: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = video_recordings)]
pub struct UpdateVideoRecording<'a> {
    pub recording_status: Option<&'a str>,
    pub recording_start: Option<Option<&'a u64>>,
    pub recording_end: Option<Option<&'a u64>>,
    pub cloud_sync_start: Option<Option<&'a u64>>,
    pub cloud_sync_end: Option<Option<&'a u64>>,
}

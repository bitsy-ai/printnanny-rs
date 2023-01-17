use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use crate::enums::Status;
use crate::schema::video_recordings;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[table_name = "video_recordings"]
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
#[table_name = "video_recordings"]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub recording_file_name: &'a str,
    pub gcode_file_name: &'a str,
}

// TODO
// #[derive(AsChangeset)]
// #[table_name = "video_recordings"]
// pub struct UpdateVideoRecording<'a> {
//     pub recording_status: Option<&'a str>,
//     pub recording_start: Option<&'a u64>,
//     pub recording_end: Option<&'a u64>,
//     pub cloud_sync_start: Option<&'a u64>,
//     pub cloud_sync_end: Option<&'a u64>,
// }

use diesel::prelude::*;

#[derive(Queryable)]
pub struct VideoRecording {
    pub id: diesel::sql_types::Uuid,
    pub recording_start: u64,
    pub recording_end: Option<u64>,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_start: u64,
    pub cloud_sync_end: Option<u64>,
}

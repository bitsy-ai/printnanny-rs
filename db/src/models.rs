use diesel::prelude::*;

#[derive(Queryable)]
pub struct VideoRecording {
    pub id: diesel::sql_types::Uuid,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub ts: u64,
    pub backup_done: bool,
}

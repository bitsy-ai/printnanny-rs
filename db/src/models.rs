use diesel::prelude::*;

#[derive(Queryable)]
pub struct VideoRecording {
    pub id: diesel::sql_types::Uuid,
    pub file_name: String,
    pub ts: u64,
    pub backup_done: bool,
}

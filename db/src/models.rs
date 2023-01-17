use diesel::prelude::*;
use diesel_enum::DbEnum;

#[derive(Debug)]
pub struct EnumError {
    msg: String,
    status: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow, DbEnum)]
#[sql_type = "VarChar"]
#[error_fn = "EnumError::not_found"]
#[error_type = "EnumError"]
pub enum Status {
    Pending,
    InProgress,
    Done,
}

#[derive(Queryable)]
pub struct VideoRecording {
    pub id: diesel::sql_types::Uuid,
    pub recording_status: Status,
    pub recording_start: Option<u64>,
    pub recording_end: Option<u64>,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: Status,
    pub cloud_sync_start: Option<u64>,
    pub cloud_sync_end: Option<u64>,
}

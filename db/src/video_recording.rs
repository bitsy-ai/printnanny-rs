use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::connection::establish_sqlite_connection;
use crate::schema::video_recordings;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Default)]
#[diesel(table_name = video_recordings)]
pub struct VideoRecording {
    pub id: String,
    pub recording_status: String,
    pub recording_start: Option<DateTime<Utc>>,
    pub recording_end: Option<DateTime<Utc>>,
    pub recording_file_name: String,
    pub gcode_file_name: Option<String>,
    pub cloud_sync_status: String,
    pub cloud_sync_start: Option<DateTime<Utc>>,
    pub cloud_sync_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = video_recordings)]
pub struct NewVideoRecording<'a> {
    pub id: &'a str,
    pub recording_file_name: &'a str,
    pub gcode_file_name: &'a str,
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

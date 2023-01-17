use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
pub mod cloud;
pub mod enums;
pub mod schema;
pub mod video_recording;

pub use diesel;

use printnanny_settings::printnanny::PrintNannySettings;

pub fn establish_sqlite_connection() -> SqliteConnection {
    let settings = PrintNannySettings::new().expect("Failed to initialize PrintNannySettings");
    let database_path = settings.paths.db();
    SqliteConnection::establish(&database_path.display().to_string())
        .expect("Failed to initialize sqlite db connection")
}

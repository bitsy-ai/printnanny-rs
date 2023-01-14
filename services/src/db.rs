use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

pub fn establish_connection() -> SqliteConnection {
    let settings = PrintNannySettings::new().expect("Failed to initialize PrintNannySettings");
    let database_path = settings.paths.db();
    SqliteConnection::establish(database_path.display())
        .expect("Failed to initialize sqlite db connection")
}

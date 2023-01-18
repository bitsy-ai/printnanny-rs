use std::error::Error;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use printnanny_settings::printnanny::PrintNannySettings;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn establish_sqlite_connection() -> SqliteConnection {
    let settings = PrintNannySettings::new().expect("Failed to initialize PrintNannySettings");
    let database_path = settings.paths.db();
    SqliteConnection::establish(&database_path.display().to_string())
        .expect("Failed to initialize sqlite db connection")
}

pub fn run_migrations() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let connection = &mut establish_sqlite_connection();
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

use std::error::Error;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn establish_sqlite_connection(database_path: &str) -> SqliteConnection {
    SqliteConnection::establish(database_path).expect("Failed to initialize sqlite db connection")
}

pub fn run_migrations(database_path: &str) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let connection = &mut establish_sqlite_connection(database_path);
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

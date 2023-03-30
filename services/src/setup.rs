use printnanny_edge_db::connection::run_migrations;
use printnanny_settings::printnanny::PrintNannySettings;

use crate::error::{IoError, ServiceError};

// one-time PrintNanyn OS setup tasks
pub async fn printnanny_os_init() -> Result<(), ServiceError> {
    let settings = PrintNannySettings::new().await?;
    // ensure directory structure exists
    settings.paths.try_init_all()?;
    let sqlite_connection = settings.paths.db().display().to_string();
    // run any pending migrations
    run_migrations(&sqlite_connection).map_err(|e| ServiceError::SQLiteMigrationError {
        msg: (*e).to_string(),
    })?;
    Ok(())
}

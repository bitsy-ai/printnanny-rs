use printnanny_edge_db::connection::run_migrations;
use printnanny_settings::printnanny::PrintNannySettings;

use crate::error::ServiceError;

// one-time PrintNanyn OS setup tasks
pub fn printnanny_os_init() -> Result<(), ServiceError> {
    let settings = PrintNannySettings::new()?;
    // ensure directory structure exists
    settings.paths.try_init_all()?;
    // run any pending migrations
    run_migrations()?;
    Ok(())
}

use std::process::{Command, Output};

use anyhow::{Ok, Result};
use printnanny_services::error::ServiceError;
use printnanny_settings::cam::CameraVideoSource;

pub struct CameraCommand;

impl CameraCommand {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        Ok(())
    }
}

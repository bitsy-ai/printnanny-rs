use std::io;
use std::io::Write;

use anyhow::{Ok, Result};
use printnanny_services::error::ServiceError;
use printnanny_settings::{cam::CameraVideoSource, SettingsFormat};

pub struct CameraCommand;

impl CameraCommand {
    pub fn handle(args: &clap::ArgMatches) -> Result<()> {
        let output = CameraVideoSource::from_libcamera_list()?;
        let f: SettingsFormat = args.value_of_t("format").unwrap();

        let v = match f {
            SettingsFormat::Json => serde_json::to_vec_pretty(&output)?,
            SettingsFormat::Toml => toml::ser::to_vec(&output)?,
            _ => todo!(),
        };
        io::stdout().write_all(&v)?;

        Ok(())
    }
}

use printnanny_services::config::{ConfigFormat, PrintNannyConfig};
use printnanny_services::error::ServiceError;
use printnanny_services::keys::PrintNannyKeys;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub struct ConfigAction;

impl ConfigAction {
    pub fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let config: PrintNannyConfig = PrintNannyConfig::new()?;
        match sub_m.subcommand() {
            Some(("init", args)) => {
                let output = args.value_of("output").unwrap();
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                config.try_init(output, &f)?
            }
            Some(("get", args)) => {
                let key = args.value_of("key").unwrap();
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let data = PrintNannyConfig::find_value(key)?;
                let v = match f {
                    ConfigFormat::Json => serde_json::to_vec_pretty(&data)?,
                    ConfigFormat::Toml => toml::ser::to_vec(&data)?,
                };
                io::stdout().write_all(&v)?;
            }
            Some(("generate-keys", args)) => {
                let path = PathBuf::from(args.value_of("output").unwrap());
                fs::create_dir_all(&path)?;
                let force_create = args.is_present("force");
                let keys = PrintNannyKeys { path, force_create };
                keys.try_generate()?
            }
            Some(("show", args)) => {
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => serde_json::to_vec_pretty(&config)?,
                    ConfigFormat::Toml => toml::ser::to_vec(&config)?,
                };
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected generate-keys|get|init|generate-keys subcommand"),
        };
        Ok(())
    }
}

pub async fn handle_check_license() -> Result<(), ServiceError> {
    let config: PrintNannyConfig = PrintNannyConfig::new()?;
    config.check_license().await?;
    Ok(())
}

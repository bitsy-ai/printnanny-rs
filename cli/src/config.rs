use log::info;
use printnanny_services::config::{ConfigFormat, PrintNannyConfig};
use printnanny_services::error::{PrintNannyConfigError, ServiceError};
use printnanny_services::printnanny_api::ApiService;
use std::io::{self, Write};

pub struct ConfigAction;

impl ConfigAction {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let config: PrintNannyConfig = PrintNannyConfig::new()?;
        match sub_m.subcommand() {
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
            Some(("setup", _args)) => {
                let config = PrintNannyConfig::new()?;
                let result = config.keys.try_generate();
                // setup action is idempotent, so KeypairExists error is non-fatal. Just log if generation was skipped
                match result {
                    Ok(_) => Ok(()),
                    Err(e) => match &e {
                        PrintNannyConfigError::KeypairExists { .. } => {
                            info!("{}", e);
                            Ok(())
                        }
                        // surface all other errors
                        _ => Err(e),
                    },
                }?;
                let mut service = ApiService::new(config)?;
                service.device_setup().await?;
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

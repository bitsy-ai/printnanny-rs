use log::info;
use printnanny_services::config::{ConfigFormat, PrintNannyConfig};
use printnanny_services::error::{PrintNannyConfigERror, PrintNannyConfigError, ServiceError};
use printnanny_services::printnanny_api::ApiService;
use std::io::{self, Write};

pub struct ConfigCommand;

impl ConfigCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let config: PrintNannyConfig = PrintNannyConfig::new()?;
        match sub_m.subcommand() {
            Some(("get", args)) => {
                let key = args.value_of("key");
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => match key {
                        Some(k) => {
                            let data = PrintNannyConfig::find_value(k)?;
                            serde_json::to_vec_pretty(&data)?
                        }
                        None => {
                            let data = PrintNannyConfig::new()?;
                            serde_json::to_vec_pretty(&data)?
                        }
                    },
                    ConfigFormat::Toml => match key {
                        Some(k) => {
                            let data = PrintNannyConfig::find_value(k)?;
                            toml::ser::to_vec(&data)?
                        }
                        None => {
                            let data = PrintNannyConfig::new()?;
                            toml::ser::to_vec(&data)?
                        }
                    },
                };
                io::stdout().write_all(&v)?;
            }
            Some(("set", args)) => {
                let key = args.value_of("key").unwrap();
                let value = args.value_of("value").unwrap();
                let figment = PrintNannyConfig::figment()?;
                let data = figment::providers::Serialized::global(key, &value);
                let figment = figment.merge(data);
                let config: PrintNannyConfig = figment.extract()?;
                config.try_save()?;
            }
            Some(("sync", _args)) => {
                let config = PrintNannyConfig::new()?;
                let mut service = ApiService::new(config)?;
                service.sync().await?;
            }
            Some(("show", args)) => {
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => serde_json::to_vec_pretty(&config)?,
                    ConfigFormat::Toml => toml::ser::to_vec(&config)?,
                };
                io::stdout().write_all(&v)?;
            }
            Some(("init", args)) => {
                let force = args.is_present("force");
                info!("PrintNannyConfig.paths {:?}", config.paths);
                config.paths.try_init_dirs()?;
                match config.paths.unpack_seed(force) {
                    Ok(r) => Ok(r),
                    Err(PrintNannyConfigError::FileExists(e)) => {
                        warn!(e);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }?;
                match config.paths.unpack_seed(force) {
                    Ok(r) => Ok(r),
                    Err(PrintNannyConfigError::FileExists(e)) => {
                        warn!(e);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            _ => panic!("Expected generate-keys|get|init|generate-keys subcommand"),
        };
        Ok(())
    }
}

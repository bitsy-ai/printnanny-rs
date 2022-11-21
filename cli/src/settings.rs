use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::settings::{ConfigFormat, PrintNannySettings};
use std::io::{self, Write};

pub struct SettingsCommand;

impl SettingsCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let config: PrintNannySettings = PrintNannySettings::new()?;
        match sub_m.subcommand() {
            Some(("get", args)) => {
                let key = args.value_of("key");
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => match key {
                        Some(k) => {
                            let data = PrintNannySettings::find_value(k)?;
                            serde_json::to_vec_pretty(&data)?
                        }
                        None => {
                            let data = PrintNannySettings::new()?;
                            serde_json::to_vec_pretty(&data)?
                        }
                    },
                    ConfigFormat::Toml => match key {
                        Some(k) => {
                            let data = PrintNannySettings::find_value(k)?;
                            toml::ser::to_vec(&data)?
                        }
                        None => {
                            let data = PrintNannySettings::new()?;
                            toml::ser::to_vec(&data)?
                        }
                    },
                };
                io::stdout().write_all(&v)?;
            }
            Some(("set", args)) => {
                let key = args.value_of("key").unwrap();
                let value = args.value_of("value").unwrap();
                let figment = PrintNannySettings::figment()?;
                let data = figment::providers::Serialized::global(key, &value);
                let figment = figment.merge(data);
                let config: PrintNannySettings = figment.extract()?;
                config.try_save()?;
            }
            Some(("show", args)) => {
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => serde_json::to_vec_pretty(&config)?,
                    ConfigFormat::Toml => toml::ser::to_vec(&config)?,
                };
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|set|show subcommand"),
        };
        Ok(())
    }
}

use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;

use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::vcs::VersionControlledSettings;
use printnanny_settings::SettingsFormat;

pub struct SettingsCommand;

impl SettingsCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<()> {
        let config: PrintNannySettings = PrintNannySettings::new().await?;
        match sub_m.subcommand() {
            Some(("clone", args)) => {
                let settings = PrintNannySettings::new().await?;

                let dir = args
                    .value_of("dir")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| settings.git.path.clone());
                settings.init_git_repo(&dir)?;
            }
            Some(("get", args)) => {
                let key = args.value_of("key");
                let f: SettingsFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    SettingsFormat::Json => match key {
                        Some(k) => {
                            let data = PrintNannySettings::find_value(k).await?;
                            serde_json::to_vec_pretty(&data)?
                        }
                        None => {
                            let data = PrintNannySettings::new().await?;
                            serde_json::to_vec_pretty(&data)?
                        }
                    },
                    SettingsFormat::Toml => match key {
                        Some(k) => {
                            let data = PrintNannySettings::find_value(k).await?;
                            toml::ser::to_vec(&data)?
                        }
                        None => {
                            let data = PrintNannySettings::new().await?;
                            toml::ser::to_vec(&data)?
                        }
                    },
                    SettingsFormat::Ini | SettingsFormat::Yaml => todo!(),
                };
                io::stdout().write_all(&v)?;
            }
            Some(("set", args)) => {
                let key = args.value_of("key").unwrap();
                let value = args.value_of("value").unwrap();
                let figment = PrintNannySettings::figment().await?;
                let data = figment::providers::Serialized::global(key, &value);
                let figment = figment.merge(data);
                let config: PrintNannySettings = figment.extract()?;
                let content = config.to_toml_string()?;
                let now = std::time::SystemTime::now();
                config
                    .save_and_commit(
                        &content,
                        Some(format!("PrintNannySettings.{} updated at {:?}", key, now)),
                    )
                    .await?;
            }
            Some(("show", args)) => {
                let f: SettingsFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    SettingsFormat::Json => serde_json::to_vec_pretty(&config)?,
                    SettingsFormat::Toml => toml::ser::to_vec(&config)?,
                    _ => unimplemented!("show command is not implemented for format: {}", f),
                };
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|set|show subcommand"),
        };
        Ok(())
    }
}

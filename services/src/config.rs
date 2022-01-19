use figment::error::Result;
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{magic::RelativePathBuf, Dict, Map, Value};
use figment::{Error as FigmentError, Figment, Metadata, Profile, Provider};
use glob::glob;
use log::{debug, error, info};
use printnanny_api_client::models;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use crate::paths::PrintNannyPath;
use crate::printnanny_api::ApiConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Config {
    pub api_config: ApiConfig,
    pub path: String,
    pub device: Option<models::Device>,
    pub user: Option<models::User>,
    pub port: i32,
}
pub struct ConfigError {}

impl Default for Config {
    fn default() -> Self {
        let api_config = ApiConfig {
            base_path: "https://print-nanny.com".into(),
            bearer_access_token: None,
        };
        let path = "/opt/printnanny/default".into();
        Config {
            api_config,
            path,
            device: None,
            user: None,
            port: 9001,
        }
    }
}

impl Config {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile.
    pub fn figment() -> Figment {
        // let profile = Profile::from_env_or("PRINTNANNY_PROFILE", Self::DEFAULT_PROFILE);
        let mut result = Figment::from(Config {
            // profile,
            ..Config::default()
        })
        .merge(Toml::file(Env::var_or(
            "PRINTNANNY_CONFIG",
            "PrintNanny.toml",
        )))
        .merge(Env::prefixed("PRINTNANNY_").global());

        info!("Loaded config from profile {:?}", result.profile());
        let path: String = result
            .find_value("path")
            .unwrap_or(Value::from(Config::default().path))
            .deserialize::<String>()
            .unwrap();

        let toml_glob = format!("{}/*.toml", &path);
        let json_glob = format!("{}/*.json", &path);

        let result = Config::read_path_glob::<Json>(&json_glob, result);
        let result = Config::read_path_glob::<Toml>(&toml_glob, result);

        result
    }

    fn read_path_glob<T: 'static + figment::providers::Format>(
        pattern: &str,
        figment: Figment,
    ) -> Figment {
        info!("Merging config from {:?}", &pattern);
        let mut result = figment.clone();
        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    info!("Merging config from {:?}", &path);
                    result = result.clone().merge(T::file(path))
                }
                Err(e) => error!("{:?}", e),
            }
        }
        result
    }

    /// Extract a `Config` from `provider`, panicking if extraction fails.
    ///
    /// # Panics
    ///
    /// If extraction fails, prints an error message indicating the failure and
    /// panics. For a version that doesn't panic, use [`Config::try_from()`].
    ///
    /// # Example
    pub fn from<T: Provider>(provider: T) -> Self {
        Self::try_from(provider).unwrap_or_else(|e| {
            error!("{:?}", e);
            panic!("aborting due to configuration error(s)")
        })
    }

    /// Attempts to extract a `Config` from `provider`, returning the result.
    ///
    /// # Example
    pub fn try_from<T: Provider>(provider: T) -> Result<Self> {
        let figment = Figment::from(provider);
        let config = figment.extract::<Self>()?;
        Ok(config)
    }
}

impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Print Nanny Config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>> {
        let map: Map<Profile, Dict> = Serialized::defaults(self).data()?;
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_env_merged() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "PrintNanny.toml",
                r#"
            name = "default"
            path = "/opt/printnanny/default"
            
            [api_config]
            base_path = "https://print-nanny.com"
            "#,
            )?;
            let figment = Config::figment();
            let config: Config = figment.extract()?;
            assert_eq!(
                config.api_config,
                ApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: None
                }
            );

            jail.set_env("PRINTNANNY_API_CONFIG.BEARER_ACCESS_TOKEN", "secret");
            let figment = Config::figment();
            let config: Config = figment.extract()?;
            assert_eq!(
                config.api_config,
                ApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: Some("secret".into())
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_config_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
            name = "local"
            path = "/home/leigh/projects/print-nanny-cli/.tmp"
            
            [api_config]
            base_path = "http://aurora:8000"
            "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = Config::figment();
            let config: Config = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            let path: String = "/home/leigh/projects/print-nanny-cli/.tmp".into();
            assert_eq!(config.path, path);
            assert_eq!(config.api_config.base_path, base_path);

            assert_eq!(
                config.api_config,
                ApiConfig {
                    base_path: base_path,
                    bearer_access_token: None
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_json_toml_glob() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Base.toml",
                r#"
            path = ".tmp"
            
            [api_config]
            base_path = "https://print-nanny.com"
            "#,
            )?;
            let tmp_directory = jail.directory();
            std::fs::create_dir(tmp_directory.join(".tmp")).unwrap();
            jail.create_file(
                ".tmp/ApiConfig.toml",
                r#"
            [api_config]
            base_path = "http://aurora:8000"
            bearer_access_token = "abc123"
            "#,
            )?;
            // jail.create_file(
            //     ".tmp/device.json",
            //     r#"
            // "#,
            // )?;
            jail.set_env("PRINTNANNY_CONFIG", "Base.toml");
            let figment = Config::figment();
            let config: Config = figment.extract()?;
            info!("Read config {:?}", config);
            // assert_eq!(config.api_config.bearer_access_token, "local");
            // assert_eq!(config.device.unwrap().id, 1);
            assert_eq!(
                config.api_config,
                ApiConfig {
                    base_path: "http://aurora:8000".into(),
                    bearer_access_token: Some("abc123".into())
                }
            );
            Ok(())
        });
    }
}

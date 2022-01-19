use figment::error::Result;
use figment::value::{magic::RelativePathBuf, Dict, Map};
use figment::{
    providers::{Env, Format, Json, Serialized, Toml},
    Error as FigmentError, Figment, Metadata, Profile, Provider,
};
use glob::glob;
use log::{error, info};
use printnanny_api_client::models;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use crate::paths::PrintNannyPath;
use crate::printnanny_api::ApiConfig;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Config {
    pub api_config: ApiConfig,
    pub profile: Profile,
    pub path: String,
    pub device: Option<models::Device>,
    pub user: Option<models::User>,
}
pub struct ConfigError {}

impl Default for Config {
    fn default() -> Self {
        let api_config = ApiConfig {
            base_path: "https://print-nanny.com".into(),
            bearer_access_token: None,
        };
        let path = "/opt/printnanny/default".into();
        let profile = "default".into();
        Config {
            api_config,
            profile,
            path,
            device: None,
            user: None,
        }
    }
}

impl Config {
    pub const LOCAL_PROFILE: Profile = Profile::const_new("local");
    pub const SANDBOX_PROFILE: Profile = Profile::const_new("sandbox");
    pub const DEFAULT_PROFILE: Profile = Profile::const_new("default");
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile.
    pub fn figment() -> Figment {
        let profile = Profile::from_env_or("PRINTNANNY_PROFILE", Self::DEFAULT_PROFILE);
        let result = Figment::from(Config {
            profile,
            ..Config::default()
        })
        .merge(Toml::file(Env::var_or("PRINTNANNY_CONFIG", "PrintNanny.toml")).nested())
        .merge(Env::prefixed("PRINTNANNY_").ignore(&["PROFILE"]).global())
        .select(Profile::from_env_or(
            "PRINTNANNY_PROFILE",
            Self::DEFAULT_PROFILE,
        ));
        info!("Using profile: {:?}", result.profile());

        let toml_glob = format!("{}/*.toml", &result.profile());
        let json_glob = format!("{}/*.json", &result.profile());

        for entry in glob(&toml_glob).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => println!("{:?}", path.display()),
                Err(e) => println!("{:?}", e),
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
        let mut config = figment.extract::<Self>()?;
        config.profile = figment.profile().clone();
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

    fn profile(&self) -> Option<Profile> {
        Some(self.profile.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_default_profiles_env_merged() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "PrintNanny.toml",
                r#"
            [default]
            name = "default"
            path = "/opt/printnanny/default"
            
            [default.api_config]
            base_path = "https://print-nanny.com"
            
            [local]
            name = "local"
            path = "/home/leigh/projects/print-nanny-cli/.tmp"
            
            [local.api_config]
            base_path = "http://aurora:8000"
            "#,
            )?;
            jail.set_env("PRINTNANNY_API_CONFIG.API_URL", "http://localhost:8000");
            let figment = Config::figment();
            let config: Config = figment.extract()?;
            assert_eq!(config.profile, "default");
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
            assert_eq!(config.profile, "default");
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
    fn test_custom_profile_selected() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "PrintNanny.toml",
                r#"
            [default]
            name = "default"
            path = "/opt/printnanny/default"
            
            [default.api_config]
            base_path = "https://print-nanny.com"
            
            [local]
            name = "local"
            path = "/home/leigh/projects/print-nanny-cli/.tmp"
            
            [local.api_config]
            base_path = "http://aurora:8000"
            "#,
            )?;
            jail.set_env("PRINTNANNY_API_CONFIG.API_URL", "http://localhost:8000");
            jail.set_env("PRINTNANNY_PROFILE", "local");
            let figment = Config::figment();
            let config: Config = figment.extract()?;
            assert_eq!(config.profile, "local");
            assert_eq!(
                config.api_config,
                ApiConfig {
                    base_path: "http://aurora:8000".into(),
                    bearer_access_token: None
                }
            );
            Ok(())
        });
    }
}

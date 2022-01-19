use serde::Deserialize;
use std::path::PathBuf;
use figment::{Figment, providers::{Format, Json, Env}};

use thiserror::Error;
use printnanny_api_client::models;

use crate::printnanny_api::ApiConfig;
use crate::paths::PrintNannyPath;

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    api_config: ApiConfig,
    prefix: String
    profile: String,
    device: Option<models::Device>,
    user: Option<models::User>,
}

struct ActiveProfile(String);

struct Profile {
    dir: PathBuf,
    name: String,
    active: bool
}

struct ConfigError {

    #[error("Expected 1 active profile, but found {count:?}, profiles marked active: {profiles:?}")]
    TooManyActiveProfiles {
        profiles: Vec<Profiles>,
        count: i32
    }
}

impl Default for Config {
    fn default() -> Self {
        let api_config = ApiConfig{
            base_path: "https://print-nanny.com".into(),
            bearer_access_token: None,
        }
        let profile = "/opt/printnanny/default".into();
        let prefix = "/opt/printnanny".into()
        Config { api_config, profile, prefix, device: None, user: None }
    }
}

impl Config {
    // Allow the configuration to be extracted from any `Provider`.
    fn from<T: Provider>(provider: T) -> Result<Config, Error> {
        Figment::from(provider).extract()
    }

    // Provide a default provider, a `Figment`.
    fn figment() -> Figment {
        use figment::providers::Env;

        // In reality, whatever the library desires.
        Figment::from(Config::default()).merge(Env::prefixed("PRINTNANNY_"))
    }

    fn list_profiles(&self) -> Result<(Vec<Profile>, ActiveProfile> {
        let path = Path::new(&self.prefix);
        let prefix_path = fs::read_dir(path)?;
        let mut profiles = Vec::new();
        let mut active_profile = ActiveProfile("default");

        // read base directory, assumes all subdirectories are profiles
        // an optional flag file .active indicates a profile is active
        for p in prefix_path {
            if p.is_dir(){
                let active = match fs::read_dir(format!("{}/.active", p.path())) {
                    Err(_) => false,
                    Some(_) => true
                }
                if active {
                    active_profile = ActiveProfile(p.filename());
                }
                let profile = Profile{
                    active,
                    dir: p.path(),
                    name: p.file_name(),
                }
                profiles.push(profile)
            } else {
                warn!("Ignoring file outside of profile directory: {:?} - move file into profile to use", p.path())
            }
        }
        // only one profile should be active
        let active_profile = profiles.iter().filter(|p| p.active);
        if active_profile.count() != 1 {
            Err(ConfigError::TooManyActiveProfiles{
                profiles: active_profile,
                count: active_profile.count()
            })
        } else {
            Ok(profiles, active_profile)
        }
    }
}
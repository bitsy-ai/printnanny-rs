use figment::error::Result;
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map, Value};
use figment::{Figment, Metadata, Profile, Provider};
use glob::glob;
use log::{error, info};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::models;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnsibleConfig {
    pub venv_dir: String,
    pub collection_name: String,
    pub collection_version: String,
}

impl Default for AnsibleConfig {
    fn default() -> Self {
        Self {
            venv_dir: "/opt/printnanny/ansible/venv".into(),
            collection_name: "bitsyai.printnanny".into(),
            collection_version: "1.4.1".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CmdConfig {
    pub queue_dir: String,
    pub success_dir: String,
    pub error_dir: String,
}

impl Default for CmdConfig {
    fn default() -> Self {
        Self {
            queue_dir: "/var/run/printnanny/cmd/queue".into(),
            success_dir: "/var/run/printnanny/cmd/success".into(),
            error_dir: "/var/run/printnanny/cmd/error".into(),
        }
    }
}

impl CmdConfig {
    pub fn add_to_queue(&self, event: models::PolymorphicEvent) -> Result<()> {
        let filename = format!("{}_{}", event.event_name, event.id);
        serde_json::to_writer(&File::create(path)?, model)?;
        info!(
            "Wrote event={:?} to file={:?} to await processing",
            event, filename
        );
        Ok(())
    }
}

impl AnsibleConfig {
    // ansible executable path
    pub fn ansible(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible")
    }
    // ansible-config executable path
    pub fn ansible_config(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-config")
    }
    // ansible-doc executable path
    pub fn ansible_doc(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-doc")
    }
    // ansible-galaxy executable path
    pub fn ansible_galaxy(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-galaxy")
    }
    // ansible-inventory executable path
    pub fn ansible_inventory(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-inventory")
    }
    // ansible-playbook executable path
    pub fn ansible_playbook(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-playbook")
    }
    // ansible-pull executable path
    pub fn ansible_pull(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-pull")
    }
    // ansible-vault executable path
    pub fn ansible_vault(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/ansible-vault")
    }
    // venv activate executable path
    pub fn venv_activate(&self) -> PathBuf {
        PathBuf::from(self.venv_dir.clone()).join("bin/activate")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiConfig {
    pub base_path: String,
    pub bearer_access_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashConfig {
    pub base_path: String,
    pub port: i32,
}

impl Default for DashConfig {
    fn default() -> Self {
        Self {
            base_path: "/".into(),
            port: 9001,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MQTTConfig {
    pub ca_certs: Vec<String>,
    pub private_key: String,
    pub public_key: String,
    pub fingerprint: String,
    pub fingerprint_algorithm: String,
    pub cipher: String,
    pub length: i32,
    pub keepalive: u64,
}

impl Default for MQTTConfig {
    fn default() -> Self {
        Self {
            private_key: "/opt/printnanny/default/keys/ec_private.pem".into(),
            public_key: "/opt/printnanny/default/keys/ec_public.pem".into(),
            fingerprint: "".into(),
            fingerprint_algorithm: "md5".into(),
            ca_certs: vec!["/opt/printnanny/default/ca-certificates".into()],
            cipher: "secp256r1".into(),
            length: 4096,
            keepalive: 300, // seconds
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyConfig {
    pub install_dir: String,
    pub runtime_dir: String,
    pub events_socket: String,
    pub ansible: AnsibleConfig,
    pub api: ApiConfig,
    pub dash: DashConfig,
    pub mqtt: MQTTConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<models::Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<models::User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janus_local: Option<models::JanusStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janus_cloud: Option<models::JanusStream>,
}
pub struct ConfigError {}

impl From<&ApiConfig> for ReqwestConfig {
    fn from(api: &ApiConfig) -> ReqwestConfig {
        ReqwestConfig {
            base_path: api.base_path.to_string(),
            bearer_access_token: api.bearer_access_token.clone(),
            ..ReqwestConfig::default()
        }
    }
}

impl Default for PrintNannyConfig {
    fn default() -> Self {
        let ansible = AnsibleConfig::default();
        let api = ApiConfig {
            base_path: "https://print-nanny.com".into(),
            bearer_access_token: None,
        };
        let install_dir = "/opt/printnanny/default".into();
        let runtime_dir = "/var/run/printnanny".into();
        let events_socket = "/var/run/printnanny/event.sock".into();
        let mqtt = MQTTConfig::default();
        let dash = DashConfig::default();
        PrintNannyConfig {
            ansible,
            api,
            dash,
            mqtt,
            install_dir,
            runtime_dir,
            events_socket,
            device: None,
            user: None,
            janus_cloud: None,
            janus_local: None,
        }
    }
}

impl PrintNannyConfig {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile
    pub fn new(config: Option<&str>) -> Result<Self> {
        let figment = Self::figment(config);
        let result = figment.extract()?;
        info!("Initialized config {:?}", result);
        Ok(result)
    }

    // intended for use with Rocket's figmment
    pub fn from_figment(config: Option<&str>, figment: Figment) -> Figment {
        figment.merge(Self::figment(config))
    }

    pub fn figment(config: Option<&str>) -> Figment {
        let result = Figment::from(Self {
            // profile,
            ..Self::default()
        })
        .merge(Toml::file(Env::var_or(
            "PRINTNANNY_CONFIG",
            "PrintNanny.toml",
        )))
        .merge(Env::prefixed("PRINTNANNY_").global());

        let result = match config {
            Some(c) => result.merge(Toml::file(c)),
            None => result,
        };
        info!(
            "Initialized PrintNannyConfig from PRINTNANNY_CONFIG and PRINTANNY_ env vars: \n {:?}",
            &result
        );

        info!("Loaded config from profile {:?}", result.profile());
        let path: String = result
            .find_value("install_dir")
            .unwrap_or_else(|_| Value::from(Self::default().install_dir))
            .deserialize::<String>()
            .unwrap();

        let toml_glob = format!("{}/*.toml", &path);
        let json_glob = format!("{}/*.json", &path);

        info!("Merging PrintNannyConfig from {}", &json_glob);
        let result = Self::read_path_glob::<Json>(&json_glob, result);
        info!("Merging PrintNannyConfig from {}", &toml_glob);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);
        info!("Finalized PrintNannyConfig: \n {:?}", result);
        result
    }

    fn read_path_glob<T: 'static + figment::providers::Format>(
        pattern: &str,
        figment: Figment,
    ) -> Figment {
        info!("Merging config from {:?}", &pattern);
        let mut result = figment;
        for entry in glob(pattern).expect("Failed to read glob pattern") {
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

    pub fn save(&self) -> Result<String> {
        // save api_config.json
        let msg = format!("Failed to serialize {:?}", self);
        let content = serde_json::to_string_pretty(&self).expect(&msg);
        let filename = format!("{}/{}", &self.install_dir, "PrintNannyLicense.json");
        let msg = format!("Unable to write file: {}", &filename);
        fs::write(&filename, content).expect(&msg);
        info!("Success! Wrote {}", &filename);
        Ok(filename)
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

impl Provider for PrintNannyConfig {
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
            install_dir = "/opt/printnanny/default"
            
            [api]
            base_path = "https://print-nanny.com"
            "#,
            )?;
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: None
                }
            );

            jail.set_env("PRINTNANNY_API.BEARER_ACCESS_TOKEN", "secret");
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
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
            install_dir = "/home/leigh/projects/print-nanny-cli/.tmp"
            
            [api]
            base_path = "http://aurora:8000"
            "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            let path: String = "/home/leigh/projects/print-nanny-cli/.tmp".into();
            assert_eq!(config.install_dir, path);
            assert_eq!(config.api.base_path, base_path);

            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: base_path,
                    bearer_access_token: None
                }
            );
            Ok(())
        });
    }
}

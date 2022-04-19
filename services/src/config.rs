use std::fs;
use std::fs::File;
use std::path::PathBuf;

use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use glob::glob;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use printnanny_api_client::models;

#[derive(Error, Debug)]
pub enum PrintNannyConfigError {
    #[error("Failed to handle invalid value {value:?}")]
    InvalidValue { value: String },
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

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
    pub fn add_to_queue(&self, event: models::PolymorphicCommand) {
        let (event_id, event_name) = match &event {
            models::PolymorphicCommand::WebRtcCommand(e) => (e.id, e.event_name.to_string()),
        };
        let filename = format!("{}/{}_{}", self.queue_dir, event_name, event_id);
        let result = serde_json::to_writer(
            &File::create(&filename).expect(&format!("Failed to create file {}", &filename)),
            &event,
        );
        match result {
            Ok(_) => info!(
                "Wrote event={:?} to file={:?} to await processing",
                event, filename
            ),
            Err(e) => error!("Failed to serialize event {:?} with error {:?}", event, e),
        }
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
pub struct DashConfig {
    pub base_url: String,
    pub base_path: String,
    pub port: i32,
}

impl Default for DashConfig {
    fn default() -> Self {
        let hostname = sys_info::hostname().unwrap_or("localhost".to_string());
        Self {
            base_url: format!("http://{}/", hostname),
            base_path: "/".into(),
            port: 9001,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MQTTConfig {
    pub private_key: String,
    pub public_key: String,
    pub fingerprint: String,
    pub fingerprint_algorithm: String,
    pub cipher: String,
    pub length: i32,
    pub keepalive: u64,
    pub ca_certs: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintNannyCloudProxy {
    pub hostname: String,
    pub base_path: String,
    pub url: String,
}

impl Default for PrintNannyCloudProxy {
    fn default() -> Self {
        let hostname = sys_info::hostname().unwrap_or("localhost".to_string());
        let base_path = "/printnanny-cloud".into();
        let url = format!("http://{}{}", hostname, base_path);
        Self {
            hostname,
            base_path,
            url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyConfig {
    pub ansible: AnsibleConfig,
    pub api: models::PrintNannyApiConfig,
    pub cmd: CmdConfig,
    pub dash: DashConfig,
    pub data_dir: PathBuf,
    pub edition: models::OsEdition,
    pub events_socket: PathBuf,
    pub firstboot_file: PathBuf,
    pub install_dir: PathBuf,
    pub mqtt: MQTTConfig,
    pub printnanny_cloud_proxy: PrintNannyCloudProxy,
    pub profile: String,
    pub runtime_dir: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<models::Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<models::User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudiot_device: Option<models::CloudiotDevice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janus_edge: Option<models::JanusEdgeStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janus_edge_request: Option<models::JanusEdgeStreamRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janus_cloud: Option<models::JanusCloudStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub octoprint_install_request: Option<models::OctoPrintInstallRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub octoprint_install: Option<models::OctoPrintInstall>,
}

const FACTORY_RESET: [&'static str; 7] = [
    "api",
    "cloudiot_device",
    "device",
    "janus_edge",
    "janus_cloud",
    "octoprint_install",
    "user",
];

impl Default for PrintNannyConfig {
    fn default() -> Self {
        let ansible = AnsibleConfig::default();
        let api = models::PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
            static_url: "https://printnanny.ai/static/".into(),
            dashboard_url: "https://printnanny.ai/dashboard/".into(),
        };
        let install_dir: PathBuf = "/opt/printnanny/profiles/default".into();
        let data_dir = install_dir.join("data").into();
        let firstboot_file = "/opt/printnanny/profiles/default/PrintNannyConfig.toml".into();
        let runtime_dir = "/var/run/printnanny".into();
        let events_socket = "/var/run/printnanny/event.sock".into();
        let mqtt = MQTTConfig::default();
        let dash = DashConfig::default();
        let cmd = CmdConfig::default();
        let profile = "default".into();
        let edition = models::OsEdition::OctoprintDesktop;
        let printnanny_cloud_proxy = PrintNannyCloudProxy::default();
        PrintNannyConfig {
            ansible,
            api,
            cmd,
            dash,
            data_dir,
            edition,
            events_socket,
            firstboot_file,
            install_dir,
            mqtt,
            printnanny_cloud_proxy,
            profile,
            runtime_dir,
            cloudiot_device: None,
            device: None,
            user: None,
            janus_cloud: None,
            janus_edge: None,
            janus_edge_request: None,
            octoprint_install_request: None,
            octoprint_install: None,
        }
    }
}

impl PrintNannyConfig {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile
    pub fn new(config: Option<&str>) -> figment::error::Result<Self> {
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
            .find_value("data_dir")
            .unwrap()
            .deserialize::<String>()
            .unwrap();

        let toml_glob = format!("{}/*.toml", &path);
        let json_glob = format!("{}/*.json", &path);

        let result = Self::read_path_glob::<Json>(&json_glob, result);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);
        info!("Finalized PrintNannyConfig: \n {:?}", result);
        result
    }

    fn read_path_glob<T: 'static + figment::providers::Format>(
        pattern: &str,
        figment: Figment,
    ) -> Figment {
        info!("Merging config from {}", &pattern);
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

    pub fn try_factory_reset(&self) -> Result<(), PrintNannyConfigError> {
        // for each key/value pair in FACTORY_RESET, remove file
        for key in FACTORY_RESET.iter() {
            let filename = format!("{}.toml", key);
            let filename = self.install_dir.join(filename);
            fs::remove_file(&filename)?;
            info!("Removed {} cache {:?}", key, filename);
        }
        Ok(())
    }

    /// Save FACTORY_RESET field as <field>.toml Figment fragments
    ///
    /// # Panics
    ///
    /// If serialization or fs write fails, prints an error message indicating the failure and
    /// panics. For a version that doesn't panic, use [`PrintNannyConfig::try_save_by_key()`].
    pub fn save_by_key(&self) {
        unimplemented!()
    }

    /// Save FACTORY_RESET field as <field>.toml Figment fragments
    ///
    /// If serialization or fs write fails, prints an error message indicating the failure
    fn try_save_by_key(&self, key: &str) -> Result<(), PrintNannyConfigError> {
        let content = match key {
            "api" => toml::Value::try_from(figment::util::map! { key => &self.api}),
            "cloudiot_device" => {
                toml::Value::try_from(figment::util::map! { key => &self.cloudiot_device})
            }
            "device" => toml::Value::try_from(figment::util::map! {key => &self.device }),
            "janus_cloud" => {
                toml::Value::try_from(figment::util::map! {key =>  &self.janus_cloud })
            }
            "janus_edge" => toml::Value::try_from(figment::util::map! {key =>  &self.janus_edge }),
            "octoprint_install" => {
                toml::Value::try_from(figment::util::map! {key =>  &self.octoprint_install })
            }
            "user" => toml::Value::try_from(figment::util::map! {key =>  &self.user }),
            _ => {
                warn!("try_save_by_key received unhandled key={:?} - serializing entire PrintNannyConfig", key);
                toml::Value::try_from(self)
            }
        }?;
        let filename = format!("{}.toml", key);
        let filename = self.data_dir.join(filename);
        fs::write(&filename, content.to_string())?;
        info!("Wrote {} to {:?}", key, filename);
        Ok(())
    }

    /// Save FACTORY_RESET fields as <field>.toml Figment fragments
    ///
    /// If extraction fails, prints an error message indicating the failure
    ///
    pub fn try_save(&self) -> Result<(), PrintNannyConfigError> {
        // for each key/value pair in FACTORY_RESET vec, write a separate .toml
        for key in FACTORY_RESET.iter() {
            self.try_save_by_key(key)?;
        }
        Ok(())
    }

    /// Save FACTORY_RESET fields as <field>.toml Figment fragments
    ///
    /// # Panics
    ///
    /// If extraction fails, prints an error message indicating the failure and
    /// panics. For a version that doesn't panic, use [`PrintNannyConfig::try_save()`].
    ///
    pub fn save(&self) {
        unimplemented!()
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
    pub fn try_from<T: Provider>(provider: T) -> figment::error::Result<Self> {
        let figment = Figment::from(provider);
        let config = figment.extract::<Self>()?;
        Ok(config)
    }
}

impl Provider for PrintNannyConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("PrintNannyConfig")
    }

    fn data(&self) -> figment::error::Result<Map<Profile, Dict>> {
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
                profile = "default"
                install_dir = "/opt/printnanny/default"
                data_dir = "/opt/printnanny/default/data"

                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
                models::PrintNannyApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: None,
                    static_url: "https://printnanny.ai/static/".into(),
                    dashboard_url: "https://printnanny.ai/dashboard/".into(),
                }
            );

            jail.set_env("PRINTNANNY_API.BEARER_ACCESS_TOKEN", "secret");
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
                models::PrintNannyApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: Some("secret".into()),
                    static_url: "https://printnanny.ai/static/".into(),
                    dashboard_url: "https://printnanny.ai/dashboard/".into(),
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_firstboot_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
                profile = "local"
                install_dir = "/opt/printnanny/default"
                data_dir = "/opt/printnanny/default/data"
                
                [api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            assert_eq!(config.install_dir, PathBuf::from("/opt/printnanny/default"));
            assert_eq!(config.api.base_path, base_path);

            assert_eq!(
                config.api,
                models::PrintNannyApiConfig {
                    base_path: base_path,
                    bearer_access_token: None,
                    static_url: "https://printnanny.ai/static/".into(),
                    dashboard_url: "https://printnanny.ai/dashboard/".into(),
                }
            );
            Ok(())
        });
    }
    #[test_log::test]
    fn test_save_fragment() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
                profile = "local"
                
                [api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");
            jail.set_env("PRINTNANNY_DATA_DIR", format!("{:?}", jail.directory()));

            let figment = PrintNannyConfig::figment(None);
            let mut config: PrintNannyConfig = figment.extract()?;
            config.install_dir = jail.directory().into();
            let expected = models::PrintNannyApiConfig {
                base_path: config.api.base_path,
                bearer_access_token: Some("secret_token".to_string()),
                static_url: "https://printnanny.ai/static/".into(),
                dashboard_url: "https://printnanny.ai/dashboard/".into(),
            };
            config.api = expected.clone();
            config.try_save().unwrap();
            let figment = PrintNannyConfig::figment(None);
            let new: PrintNannyConfig = figment.extract()?;
            assert_eq!(new.api, expected);
            Ok(())
        });
    }
}

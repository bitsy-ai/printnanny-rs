use std::fs;
use std::fs::File;
use std::path::PathBuf;

use clap::{ArgEnum, PossibleValue};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use glob::glob;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

use super::error::{PrintNannyConfigError, ServiceError};
use super::keys::PrintNannyKeys;
use super::octoprint::OctoPrintConfig;
use super::paths::{PrintNannyPaths, PRINTNANNY_CONFIG_DEFAULT};
use super::printnanny_api::ApiService;
use printnanny_api_client::models;

const FACTORY_RESET: [&'static str; 3] = ["api", "device", "octoprint"];

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum ConfigFormat {
    Json,
    Toml,
}

impl ConfigFormat {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        ConfigFormat::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for ConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for ConfigFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
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
    pub cmd: PathBuf,
    pub cipher: String,
    pub keepalive: u64,
    pub ca_certs: Vec<String>,
}

impl Default for MQTTConfig {
    fn default() -> Self {
        Self {
            cmd: "/var/run/printnanny/cmd".into(),
            ca_certs: vec![
                "/etc/ca-certificates/gtsltsr.crt".into(),
                "/etc/ca-certificates/GSR4.crt".into(),
            ],
            cipher: "secp256r1".into(),
            keepalive: 300, // seconds
        }
    }
}

impl MQTTConfig {
    pub fn cmd_queue(&self) -> PathBuf {
        self.cmd.join("queue")
    }
    pub fn cmd_error(&self) -> PathBuf {
        self.cmd.join("error")
    }
    pub fn cmd_success(&self) -> PathBuf {
        self.cmd.join("success")
    }
    pub fn enqueue_cmd(&self, event: models::PolymorphicCommand) {
        let (event_id, event_name) = match &event {
            models::PolymorphicCommand::WebRtcCommand(e) => (e.id, e.event_name.to_string()),
        };
        let filename = format!("{:?}/{}_{}", self.cmd_queue(), event_name, event_id);
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
    pub printnanny_cloud_proxy: PrintNannyCloudProxy,

    #[serde(skip_serializing_if = "Option::is_none")]
    // generic device data present on all Print Nanny OS editions
    pub device: Option<models::Device>,
    // edition-specific data and settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub octoprint: Option<OctoPrintConfig>,
    pub paths: PrintNannyPaths,
    pub api: models::PrintNannyApiConfig,
    pub dash: DashConfig,
    pub mqtt: MQTTConfig,
    pub keys: PrintNannyKeys,
}

impl Default for PrintNannyConfig {
    fn default() -> Self {
        let api = models::PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
            static_url: "https://printnanny.ai/static/".into(),
            dashboard_url: "https://printnanny.ai/dashboard/".into(),
        };

        let paths = PrintNannyPaths::default();
        let mqtt = MQTTConfig::default();
        let dash = DashConfig::default();
        let printnanny_cloud_proxy = PrintNannyCloudProxy::default();
        let keys = PrintNannyKeys::default();
        let octoprint = None;
        PrintNannyConfig {
            api,
            dash,
            mqtt,
            paths,
            printnanny_cloud_proxy,
            keys,
            octoprint,
            device: None,
        }
    }
}

impl PrintNannyConfig {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile
    pub fn new() -> Result<Self, ServiceError> {
        let figment = Self::figment()?;
        let result = figment.extract()?;
        info!("Initialized config {:?}", result);
        Ok(result)
    }

    pub async fn check_license(&self) -> Result<(), ServiceError> {
        match PathBuf::from(&self.paths.license).exists() {
            true => Ok(()),
            false => Err(PrintNannyConfigError::LicenseMissing {
                path: self
                    .paths
                    .license
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            }),
        }?;
        info!("Loaded license from {:?}", &self.paths.license);
        let mut api_service = ApiService::new(self.clone())?;
        api_service.device_setup().await?;
        Ok(())
    }

    pub fn find_value(key: &str) -> Result<figment::value::Value, ServiceError> {
        let figment = Self::figment()?;
        Ok(figment.find_value(key)?)
    }

    // intended for use with Rocket's figmment
    pub fn from_figment(figment: Figment) -> Figment {
        figment.merge(Self::figment().unwrap())
    }

    pub fn figment() -> Result<Figment, PrintNannyConfigError> {
        let result = Figment::from(Self { ..Self::default() })
            .merge(Toml::file(Env::var_or(
                "PRINTNANNY_CONFIG",
                PRINTNANNY_CONFIG_DEFAULT,
            )))
            .merge(Env::prefixed("PRINTNANNY_").global());

        let confd_path: String = result
            .find_value("paths.confd")
            .unwrap()
            .deserialize::<String>()
            .unwrap();
        let license_json: String = result
            .find_value("paths.license")
            .unwrap()
            .deserialize::<String>()
            .unwrap();

        // merge license.json contents
        let result = result.merge(Json::file(&license_json));

        let toml_glob = format!("{}/*.toml", &confd_path);
        let json_glob = format!("{}/*.json", &confd_path);

        let result = Self::read_path_glob::<Json>(&json_glob, result);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);
        info!("Finalized PrintNannyConfig: \n {:?}", result);
        Ok(result)
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
            let filename = self.paths.data().join(filename);
            fs::remove_file(&filename)?;
            info!("Removed {} data {:?}", key, filename);
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
    pub fn try_save_by_key(&self, key: &str) -> Result<PathBuf, PrintNannyConfigError> {
        let filename = format!("{}.toml", key);
        let filename = self.paths.confd.join(filename);
        self.try_save_fragment(key, &filename)?;
        info!("Saved config fragment: {:?}", &filename);
        Ok(filename)
    }

    pub fn try_save_fragment(
        &self,
        key: &str,
        filename: &PathBuf,
    ) -> Result<(), PrintNannyConfigError> {
        let content = match key {
            "api" => toml::Value::try_from(figment::util::map! { key => &self.api}),

            "device" => toml::Value::try_from(figment::util::map! {key => &self.device }),
            "octoprint" => toml::Value::try_from(figment::util::map! {key =>  &self.octoprint }),
            _ => {
                warn!("try_save_fragment received unhandled key={:?} - serializing entire PrintNannyConfig", key);
                toml::Value::try_from(self)
            }
        }?;
        info!("Saving {}.toml to {:?}", &key, &filename);
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

    // Save ::Default() to output file
    pub fn try_init(
        &self,
        filename: &str,
        format: &ConfigFormat,
    ) -> Result<(), PrintNannyConfigError> {
        let content: String = match format {
            ConfigFormat::Json => serde_json::to_string_pretty(self)?,
            ConfigFormat::Toml => toml::ser::to_string_pretty(self)?,
        };
        fs::write(&filename, content.to_string())?;
        Ok(())
    }

    // Move license.json from boot partition to conf.d directory
    pub fn try_copy_license(&self) -> Result<(), ServiceError> {
        if self.paths.license.exists() {
            info!("Copying {:?} to {:?}", self.paths.license, self.paths.confd);
            fs::copy(&self.paths.license, self.paths.confd.join("license.json"))?;
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
    use crate::paths::PRINTNANNY_CONFIG_FILENAME;
    #[test_log::test]
    fn test_paths() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                r#"
                profile = "default"

                [paths]
                install = "/opt/printnanny/default"
                data = "/opt/printnanny/default/data"
                
                [octoprint]
                base_path = "/home/octoprint/.octoprint"
                python = "/usr/bin/python3"
                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.octoprint.unwrap().python,
                PathBuf::from("/usr/bin/python3")
            );
            Ok(())
        });
    }
    #[test_log::test]
    fn test_env_merged() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                r#"

                [paths]
                install = "/opt/printnanny/default"
                data = "/opt/printnanny/default/data"

                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let config = PrintNannyConfig::new().unwrap();
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
            let figment = PrintNannyConfig::figment().unwrap();
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
    fn test_custom_confd() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
                profile = "local"

                [paths]
                confd = ".tmp/"
                
                [api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            assert_eq!(config.paths.confd, PathBuf::from(".tmp/"));
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
            jail.set_env("PRINTNANNY_PATHS.confd", format!("{:?}", jail.directory()));

            let figment = PrintNannyConfig::figment().unwrap();
            let mut config: PrintNannyConfig = figment.extract()?;
            config.paths.etc = jail.directory().into();

            let expected = models::PrintNannyApiConfig {
                base_path: config.api.base_path,
                bearer_access_token: Some("secret_token".to_string()),
                static_url: "https://printnanny.ai/static/".into(),
                dashboard_url: "https://printnanny.ai/dashboard/".into(),
            };
            config.api = expected.clone();
            config.try_save().unwrap();
            let figment = PrintNannyConfig::figment().unwrap();
            let new: PrintNannyConfig = figment.extract()?;
            assert_eq!(new.api, expected);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_find_value() {
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
            jail.set_env("PRINTNANNY_PATHS.confd", format!("{:?}", jail.directory()));

            let expected: Option<String> = Some("http://aurora:8000".into());
            let value: Option<String> = PrintNannyConfig::find_value("api.base_path")
                .unwrap()
                .into_string();
            assert_eq!(value, expected);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_os_release() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                r#"
                profile = "local"
                [api]
                base_path = "http://aurora:8000"
                
                "#,
            )?;
            jail.create_file(
                "os-release",
                r#"
ID=printnanny
ID_LIKE="BitsyLinux"
BUILD_ID="2022-06-18T18:46:49Z"
NAME="PrintNanny Linux"
VERSION="0.1.2 (Amber)"
VERSION_ID=0.1.2
PRETTY_NAME="PrintNanny Linux 0.1.2 (Amber)"
DISTRO_CODENAME="Amber"
HOME_URL="https://printnanny.ai"
BUG_REPORT_URL="https://github.com/bitsy-ai/printnanny-os/issues"
YOCTO_VERSION="4.0.1"
YOCTO_CODENAME="Kirkstone"
SDK_VERSION="0.1.2"
VARIANT="PrintNanny OctoPrint Edition"
VARIANT_ID=printnanny-octoprint
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            jail.set_env(
                "PRINTNANNY_PATHS.os_release",
                format!("{:?}", jail.directory().join("os-release")),
            );

            let config = PrintNannyConfig::new().unwrap();
            let os_release = config.paths.load_os_release().unwrap();
            // let unknown_value = Value::from("unknown");
            // let os_build_id: String = os_release
            //     .get("BUILD_ID")
            //     .unwrap_or(&unknown_value)
            //     .as_str()
            //     .unwrap()
            //     .into();
            assert_eq!("2022-06-18T18:46:49Z".to_string(), os_release.build_id);
            Ok(())
        });
    }
}

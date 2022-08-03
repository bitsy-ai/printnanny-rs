use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

use clap::{ArgEnum, PossibleValue};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use file_lock::{FileLock, FileOptions};
use glob::glob;
use log::{error, info};
use serde::{Deserialize, Serialize};

use super::error::PrintNannyConfigError;
use super::octoprint::OctoPrintConfig;
use super::paths::{PrintNannyPaths, PRINTNANNY_CONFIG_DEFAULT};
use printnanny_api_client::models;

// FACTORY_RESET holds the struct field names of PrintNannyConfig
// each member of FACTORY_RESET is written to a separate config fragment under /etc/printnanny/conf.d
// as the name implies, this const is used for performing a reset of any config data modified from defaults
const FACTORY_RESET: [&str; 4] = ["api", "pi", "octoprint", "printnanny_cloud_proxy"];

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
        let hostname = sys_info::hostname().unwrap_or_else(|_| "localhost".to_string());
        Self {
            base_url: format!("http://{}/", hostname),
            base_path: "/".into(),
            port: 9001,
        }
    }
}

// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct MQTTConfig {
//     pub cmd: PathBuf,
//     pub cipher: String,
//     pub keepalive: u64,
//     pub ca_certs: Vec<String>,
// }

// impl Default for MQTTConfig {
//     fn default() -> Self {
//         Self {
//             cmd: "/var/run/printnanny/cmd".into(),
//             ca_certs: vec![
//                 "/etc/ca-certificates/gtsltsr.crt".into(),
//                 "/etc/ca-certificates/GSR4.crt".into(),
//             ],
//             cipher: "secp256r1".into(),
//             keepalive: 300, // seconds
//         }
//     }
// }

// impl MQTTConfig {
//     pub fn cmd_queue(&self) -> PathBuf {
//         self.cmd.join("queue")
//     }
//     pub fn cmd_error(&self) -> PathBuf {
//         self.cmd.join("error")
//     }
//     pub fn cmd_success(&self) -> PathBuf {
//         self.cmd.join("success")
//     }
//     pub fn enqueue_cmd(&self, event: models::PolymorphicCommand) {
//         let (event_id, event_name) = match &event {
//             models::PolymorphicCommand::WebRtcCommand(e) => (e.id, e.event_name.to_string()),
//         };
//         let filename = format!("{:?}/{}_{}", self.cmd_queue(), event_name, event_id);
//         let result = serde_json::to_writer(
//             &File::create(&filename).expect(&format!("Failed to create file {}", &filename)),
//             &event,
//         );
//         match result {
//             Ok(_) => info!(
//                 "Wrote event={:?} to file={:?} to await processing",
//                 event, filename
//             ),
//             Err(e) => error!("Failed to serialize event {:?} with error {:?}", event, e),
//         }
//     }
// }

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
    pub pi: Option<models::Pi>,
    // edition-specific data and settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub octoprint: Option<OctoPrintConfig>,
    pub paths: PrintNannyPaths,
    pub api: models::PrintNannyApiConfig,
    pub dash: DashConfig,
    // pub mqtt: MQTTConfig,
    // pub keys: PrintNannyKeys,
}

impl Default for PrintNannyConfig {
    fn default() -> Self {
        // default to unauthenticated api config, until api creds are unpacked from seed archive
        let api = models::PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
        };

        let paths = PrintNannyPaths::default();
        // let mqtt = MQTTConfig::default();
        let dash = DashConfig::default();
        let printnanny_cloud_proxy = PrintNannyCloudProxy::default();
        // let keys = PrintNannyKeys::default();
        PrintNannyConfig {
            api,
            dash,
            paths,
            printnanny_cloud_proxy,
            octoprint: None,
            pi: None,
        }
    }
}

impl PrintNannyConfig {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile
    pub fn new() -> Result<Self, PrintNannyConfigError> {
        let figment = Self::figment()?;
        let result = figment.extract()?;
        info!("Initialized config {:?}", result);
        Ok(result)
    }
    pub fn find_value(key: &str) -> Result<figment::value::Value, PrintNannyConfigError> {
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
            // allow nested environment variables:
            // PRINTNANNY_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_").split("__"));

        let etc_path: String = result
            .find_value("paths.etc")
            .unwrap()
            .deserialize::<String>()
            .unwrap();

        let confd_path = PrintNannyPaths {
            etc: PathBuf::from(etc_path),
            ..PrintNannyPaths::default()
        }
        .confd();

        let toml_glob = format!("{}/*.toml", &confd_path.display());
        let json_glob = format!("{}/*.json", &confd_path.display());

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

    pub fn is_authenticated(&self) -> bool {
        let pi_is_registered = match &self.pi {
            Some(_) => true,
            None => false,
        };

        let api_auth_set = match &self.api.bearer_access_token {
            Some(_) => true,
            None => false,
        };

        return pi_is_registered && api_auth_set && self.paths.nats_creds().exists();
    }

    pub fn try_factory_reset(&self) -> Result<(), PrintNannyConfigError> {
        // for each key/value pair in FACTORY_RESET, remove file
        for key in FACTORY_RESET.iter() {
            let filename = format!("{}.toml", key);
            let filename = self.paths.confd().join(filename);
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
        let filename = self.paths.confd().join(filename);
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
            "api" => Ok(toml::Value::try_from(
                figment::util::map! { key => &self.api},
            )?),
            "pi" => Ok(toml::Value::try_from(
                figment::util::map! {key => &self.pi },
            )?),
            "octoprint" => Ok(toml::Value::try_from(
                figment::util::map! {key =>  &self.octoprint },
            )?),
            "printnanny_cloud_proxy" => Ok(toml::Value::try_from(
                figment::util::map! {key =>  &self.printnanny_cloud_proxy },
            )?),
            // "paths" => Ok(toml::Value::try_from(
            //     figment::util::map! {key =>  &self.paths },
            // )?),
            // "mqtt" => Ok(toml::Value::try_from(
            //     figment::util::map! {key =>  &self.mqtt },
            // )?),
            // "keys" => Ok(toml::Value::try_from(
            //     figment::util::map! {key =>  &self.keys },
            // )?),
            _ => Err(PrintNannyConfigError::InvalidValue { value: key.into() }),
        }?
        .to_string();

        info!("Saving {}.toml to {:?}", &key, &filename);

        // lock fragment for writing
        let lock_for_writing = FileOptions::new().write(true).create(true).truncate(true);
        let mut filelock = FileLock::lock(&filename, true, lock_for_writing)?;
        filelock.file.write_all(content.as_bytes())?;
        // Manually unlocking is optional as we unlock on Drop
        filelock.unlock()?;
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
        fs::write(&filename, content)?;
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
    fn test_nested_env_var() {
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
            let expected = PathBuf::from("testing");
            jail.set_env("PRINTNANNY_OCTOPRINT__BASE_PATH", &expected.display());
            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(config.octoprint.unwrap().base_path, expected);
            Ok(())
        });
    }

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
                etc = ".tmp"
                
                [api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            assert_eq!(config.paths.confd(), PathBuf::from(".tmp/conf.d"));
            assert_eq!(config.api.base_path, base_path);

            assert_eq!(
                config.api,
                models::PrintNannyApiConfig {
                    base_path: base_path,
                    bearer_access_token: None,
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_save_fragment() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();
            jail.create_file(
                "Local.toml",
                &format!(
                    r#"
                profile = "local"
                [api]
                base_path = "http://aurora:8000"

                [paths]
                etc = "{}/etc"
                run = "{}/run"
                log = "{}/log"
                "#,
                    output, output, output
                ),
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment().unwrap();
            let mut config: PrintNannyConfig = figment.extract()?;
            config.paths.try_init_dirs().unwrap();

            let expected = models::PrintNannyApiConfig {
                base_path: config.api.base_path,
                bearer_access_token: Some("secret_token".to_string()),
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
                "PRINTNANNY_PATHS__OS_RELEASE",
                format!("{:?}", jail.directory().join("os-release")),
            );

            let config = PrintNannyConfig::new().unwrap();
            let os_release = config.paths.load_os_release().unwrap();
            assert_eq!("2022-06-18T18:46:49Z".to_string(), os_release.build_id);
            Ok(())
        });
    }
}

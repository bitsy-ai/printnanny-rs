use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

use clap::{ArgEnum, ArgMatches, PossibleValue};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use file_lock::{FileLock, FileOptions};
use glob::glob;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::error::ServiceError;

use super::error::PrintNannyConfigError;
use super::paths::{PrintNannyPaths, DEFAULT_PRINTNANNY_CONFIG};
use super::printnanny_api::ApiService;
use printnanny_api_client::models;

// FACTORY_RESET holds the struct field names of PrintNannyCloudConfig
// each member of FACTORY_RESET is written to a separate config fragment under /etc/printnanny/conf.d
// as the name implies, this const is used for performing a reset of any config data modified from defaults
const FACTORY_RESET: [&str; 3] = ["cloud", "systemd_units", "vision"];

lazy_static! {
    static ref DEFAULT_SYSTEMD_UNITS: HashMap<String, SystemdUnit> = {
        let mut m = HashMap::new();

        // printnanny-vision.service
        m.insert(
            "printnanny-vision.service".to_string(),
            SystemdUnit {
                unit: "printnanny-vision.service".to_string(),
                enabled: true,
            },
        );

        // octoprint.service
        m.insert(
            "octoprint.service".to_string(),
            SystemdUnit {
                unit: "octoprint.service".to_string(),
                enabled: true,
            },
        );

        // mainsail.service
        m.insert(
            "mainsail.service".to_string(),
            SystemdUnit {
                unit: "mansail.service".to_string(),
                enabled: false,
            },
        );
        m
    };
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum ConfigFormat {
    Json,
    Toml,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct NatsConfig {
    pub uri: String,
    pub require_tls: bool,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            uri: "nats://localhost:4222".to_string(),
            require_tls: false,
        }
    }
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
pub struct PrintNannyCloudProxy {
    pub hostname: String,
    pub base_path: String,
    pub url: String,
}

impl Default for PrintNannyCloudProxy {
    fn default() -> Self {
        let hostname = sys_info::hostname().unwrap_or_else(|_| "localhost".to_string());
        let base_path = "/printnanny-cloud".into();
        let url = format!("http://{}{}", hostname, base_path);
        Self {
            hostname,
            base_path,
            url,
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum, Deserialize, Serialize, PartialEq)]
pub enum VideoSrcType {
    File,
    Device,
    Uri,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyCloudConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pi: Option<models::Pi>,
    pub api: models::PrintNannyApiConfig,
}

impl Default for PrintNannyCloudConfig {
    fn default() -> Self {
        // default to unauthenticated api config, until api creds are unpacked from seed archive
        let api = models::PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
        };
        PrintNannyCloudConfig { api, pi: None }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TfliteModelConfig {
    pub label_file: String,
    pub model_file: String,
    pub nms_threshold: i32,
    pub tensor_batch_size: i32,
    pub tensor_channels: i32,
    pub tensor_height: i32,
    pub tensor_width: i32,
}

impl Default for TfliteModelConfig {
    fn default() -> Self {
        Self {
            label_file: "/etc/printnanny/data/dict.txt".into(),
            model_file: "/etc/printnanny/data/model.tflite".into(),
            nms_threshold: 50,
            tensor_batch_size: 40,
            tensor_channels: 3,
            tensor_height: 320,
            tensor_width: 320,
        }
    }
}

impl From<&ArgMatches> for TfliteModelConfig {
    fn from(args: &ArgMatches) -> Self {
        let label_file = args
            .value_of("label_file")
            .expect("--label-file is required")
            .into();
        let model_file = args
            .value_of("model_file")
            .expect("--model-file is required")
            .into();
        let tensor_batch_size: i32 = args
            .value_of_t::<i32>("tensor_batch_size")
            .expect("--tensor-batch-size must be an integer");

        let tensor_height: i32 = args
            .value_of_t::<i32>("tensor_height")
            .expect("--tensor-height must be an integer");

        let tensor_width: i32 = args
            .value_of_t::<i32>("tensor_width")
            .expect("--tensor-width must be an integer");

        let tensor_channels: i32 = args
            .value_of_t::<i32>("tensor_channels")
            .expect("--tensor-channels must be an integer");

        let nms_threshold: i32 = args
            .value_of_t::<i32>("nms_threshold")
            .expect("--nms-threshold must be an integer");

        Self {
            label_file,
            model_file,
            nms_threshold,
            tensor_batch_size,
            tensor_channels,
            tensor_height,
            tensor_width,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyGstPipelineConfig {
    pub video_src: String,
    pub preview: bool,
    pub tflite_model: TfliteModelConfig,
    pub udp_port: i32,
    pub video_height: i32,
    pub video_stream_src: VideoSrcType,
    pub video_width: i32,
}

impl Default for PrintNannyGstPipelineConfig {
    fn default() -> Self {
        let video_src = "/dev/video0".into();
        let preview = false;
        let tflite_model = TfliteModelConfig::default();
        let udp_port = 20001;
        let video_stream_src = VideoSrcType::Device;
        let video_height = 480;
        let video_width = 640;
        Self {
            video_src,
            tflite_model,
            video_stream_src,
            video_height,
            video_width,
            udp_port,
            preview,
        }
    }
}

impl From<&ArgMatches> for PrintNannyGstPipelineConfig {
    fn from(args: &ArgMatches) -> Self {
        let tflite_model = TfliteModelConfig::from(args);

        let video_stream_src: &VideoSrcType = args
            .get_one::<VideoSrcType>("video_src_type")
            .expect("--video-src-type");

        let video_src = args
            .value_of("video_src")
            .expect("--video-src is required")
            .into();
        let video_height: i32 = args
            .value_of_t::<i32>("video_height")
            .expect("--video-height must be an integer");

        let video_width: i32 = args
            .value_of_t::<i32>("video_width")
            .expect("--video-width must be an integer");

        let udp_port: i32 = args
            .value_of_t("udp_port")
            .expect("--udp-port must be an integer");

        let preview = args.is_present("preview");

        Self {
            tflite_model,
            preview,
            video_src,
            video_height,
            video_width,
            video_stream_src: video_stream_src.clone(),
            udp_port,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemdUnit {
    unit: String,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyConfig {
    pub vision: PrintNannyGstPipelineConfig,
    pub cloud: PrintNannyCloudConfig,
    pub paths: PrintNannyPaths,
    pub systemd_units: HashMap<String, SystemdUnit>,
}

impl Default for PrintNannyConfig {
    fn default() -> Self {
        let paths = PrintNannyPaths::default();
        PrintNannyConfig {
            paths,
            cloud: PrintNannyCloudConfig::default(),
            systemd_units: DEFAULT_SYSTEMD_UNITS.clone(),
            vision: PrintNannyGstPipelineConfig::default(),
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
        debug!("Initialized config {:?}", result);
        Ok(result)
    }
    pub fn find_value(key: &str) -> Result<figment::value::Value, PrintNannyConfigError> {
        let figment = Self::figment()?;
        Ok(figment.find_value(key)?)
    }

    pub async fn sync(self) -> Result<(), ServiceError> {
        let mut service = ApiService::new(self)?;
        service.sync().await
    }

    // intended for use with Rocket's figmment
    pub fn from_figment(figment: Figment) -> Figment {
        figment.merge(Self::figment().unwrap())
    }

    // Load configuration with the following order of precedence:
    //
    // 1) Environment variables prefixed with PRINTNANNY_ (highest)
    // Example:
    //    PRINTNANNY_NATS_APP__NATS_URI="nats://localhost:4222" will override all other nats_uri settings
    //
    // 2) PRINTNANNY_CONFIG .toml. configuration file
    //
    // 3) Glob pattern of .toml and .json configuration file fragments in conf.d folder
    //
    // 4) Defaults (from implement Default)

    pub fn check_file_from_env_var(var: &str) -> Result<(), PrintNannyConfigError> {
        // try reading env var
        match env::var(var) {
            Ok(value) => {
                // check that value exists
                let path = PathBuf::from(value);
                match path.exists() {
                    true => Ok(()),
                    false => Err(PrintNannyConfigError::ConfigFileNotFound { path }),
                }
            }
            Err(_) => {
                warn!(
                    "PRINTNANNY_CONFIG not set. Initializing from PrintNannyCloudConfig::default()"
                );
                Ok(())
            }
        }
    }

    pub fn figment() -> Result<Figment, PrintNannyConfigError> {
        // merge file in PRINTNANNY_CONFIG env var (if set)
        let result = Figment::from(Self { ..Self::default() })
            .merge(Toml::file(Env::var_or(
                "PRINTNANNY_CONFIG",
                DEFAULT_PRINTNANNY_CONFIG,
            )))
            // allow nested environment variables:
            // PRINTNANNY_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_").split("__"));

        // extract paths, to load conf.d fragments
        let etc_path: String = result
            .find_value("paths.etc")
            .unwrap()
            .deserialize::<String>()
            .unwrap();
        let paths = PrintNannyPaths {
            etc: PathBuf::from(etc_path),
            ..PrintNannyPaths::default()
        };

        let confd_path = paths.confd();
        let license_path = paths.license();

        // if license.json exists, load config from license.json
        let result = match license_path.exists() {
            true => result.merge(Json::file(&license_path)),
            false => result,
        };

        let toml_glob = format!("{}/*.toml", &confd_path.display());
        let json_glob = format!("{}/*.json", &confd_path.display());

        let result = Self::read_path_glob::<Json>(&json_glob, result);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);

        // if PRINTNANNY_CONFIG env var is set, check file exists and is readable
        Self::check_file_from_env_var("PRINTNANNY_CONFIG")?;

        // finally, re-merge PRINTNANNY_CONFIG and PRINTNANNY_ENV so these values take highest precedence
        let result = result
            .merge(Toml::file(Env::var_or(
                "PRINTNANNY_CONFIG",
                DEFAULT_PRINTNANNY_CONFIG,
            )))
            // allow nested environment variables:
            // PRINTNANNY_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_").split("__"));

        info!("Finalized PrintNannyCloudConfig: \n {:?}", result);
        Ok(result)
    }

    fn read_path_glob<T: 'static + figment::providers::Format>(
        pattern: &str,
        figment: Figment,
    ) -> Figment {
        debug!("Merging config from {}", &pattern);
        let mut result = figment;
        for entry in glob(pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let key = path.file_stem().unwrap().to_str().unwrap();
                    debug!("Merging key={} config from {}", &key, &path.display());
                    result = result.clone().merge(T::file(&path));
                }
                Err(e) => error!("{:?}", e),
            }
        }
        result
    }

    pub fn try_check_license(&self) -> Result<(), PrintNannyConfigError> {
        match &self.cloud.pi {
            Some(_) => Ok(()),
            None => Err(PrintNannyConfigError::LicenseMissing {
                path: "pi".to_string(),
            }),
        }?;

        match &self.cloud.api.bearer_access_token {
            Some(_) => Ok(()),
            None => Err(PrintNannyConfigError::LicenseMissing {
                path: "api.bearer_access_token".to_string(),
            }),
        }?;

        match self.paths.cloud_nats_creds().exists() {
            true => Ok(()),
            false => Err(PrintNannyConfigError::LicenseMissing {
                path: self.paths.cloud_nats_creds().display().to_string(),
            }),
        }?;

        match self.cloud.pi.as_ref().unwrap().nats_app {
            Some(_) => Ok(()),
            None => Err(PrintNannyConfigError::LicenseMissing {
                path: "pi.nats_app".to_string(),
            }),
        }?;
        Ok(())
    }

    pub fn try_factory_reset(&self) -> Result<(), PrintNannyConfigError> {
        // for each key/value pair in FACTORY_RESET, remove file
        for key in FACTORY_RESET.iter() {
            let filename = format!("{}.json", key);
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
    /// panics. For a version that doesn't panic, use [`PrintNannyCloudConfig::try_save_by_key()`].
    pub fn save_by_key(&self) {
        unimplemented!()
    }

    /// Save FACTORY_RESET field as <field>.toml Figment fragments
    ///
    /// If serialization or fs write fails, prints an error message indicating the failure
    pub fn try_save_by_key(&self, key: &str) -> Result<PathBuf, PrintNannyConfigError> {
        let filename = format!("{}.json", key);
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
            "cloud" => Ok(serde_json::to_string(
                &figment::util::map! {key => &self.cloud},
            )?),
            "systemd_units" => Ok(serde_json::to_string(
                &figment::util::map! {key => &self.systemd_units},
            )?),
            "vision" => Ok(serde_json::to_string(
                &figment::util::map! {key => &self.vision},
            )?),
            _ => Err(PrintNannyConfigError::InvalidValue { value: key.into() }),
        }?;

        info!("Saving {}.json to {:?}", &key, &filename);

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
            match self.try_save_by_key(key) {
                Ok(_) => (),
                Err(e) => error!("{}", e),
            }
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
    /// panics. For a version that doesn't panic, use [`PrintNannyCloudConfig::try_save()`].
    ///
    pub fn save(&self) {
        return self.try_save().expect("Failed to save PrintNannyConfig");
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
    fn test_config_file_not_found() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let result = PrintNannyConfig::figment();
            assert!(result.is_err());
            // assert_eq!(result, expected);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_nested_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                r#"
                profile = "default"

                [paths]
                etc = "/this/etc/path/gets/overridden"
                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let expected = PathBuf::from("testing");
            jail.set_env("PRINTNANNY_PATHS__ETC", &expected.display());
            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(config.paths.etc, expected);
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
                etc = "/opt/printnanny/etc"                
                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.paths.data(),
                PathBuf::from("/opt/printnanny/etc/data")
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

                
                [cloud.api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);
            let config = PrintNannyConfig::new().unwrap();
            assert_eq!(
                config.cloud.api,
                models::PrintNannyApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: None,
                }
            );
            jail.set_env("PRINTNANNY_CLOUD__API.BEARER_ACCESS_TOKEN", "secret");
            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.cloud.api,
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
                
                [cloud.api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment().unwrap();
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            assert_eq!(config.paths.confd(), PathBuf::from(".tmp/conf.d"));
            assert_eq!(config.cloud.api.base_path, base_path);

            assert_eq!(
                config.cloud.api,
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
                [cloud.api]
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
                base_path: config.cloud.api.base_path,
                bearer_access_token: Some("secret_token".to_string()),
            };
            config.cloud.api = expected.clone();
            config.try_save().unwrap();
            let figment = PrintNannyConfig::figment().unwrap();
            let new: PrintNannyConfig = figment.extract()?;
            assert_eq!(new.cloud.api, expected);
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
                [cloud.api]
                base_path = "http://aurora:8000"
                "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");
            jail.set_env("PRINTNANNY_PATHS.confd", format!("{:?}", jail.directory()));

            let expected: Option<String> = Some("http://aurora:8000".into());
            let value: Option<String> = PrintNannyConfig::find_value("cloud.api.base_path")
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

    #[test_log::test]
    fn test_vision_gst_pipeline_conf() {
        figment::Jail::expect_with(|jail| {
            let video_src = "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4";
            let video_stream_src = "Uri";
            let output = jail.directory().to_str().unwrap();

            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                &format!(
                    r#"
                profile = "local"
                [paths]
                etc = "{output}/etc"
                run = "{output}/run"
                log = "{output}/log"

                [vision]
                video_stream_src = "{video_stream_src}"
                video_src = "{video_src}"
                [vision.tflite_model]
                tensor_height = 400
                tensor_width = 400
                
                "#,
                    video_src = video_src,
                    video_stream_src = video_stream_src,
                    output = output
                ),
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);

            let mut config = PrintNannyConfig::new().unwrap();
            assert_eq!(config.vision.video_stream_src, VideoSrcType::Uri);
            assert_eq!(config.vision.tflite_model.tensor_height, 400);
            assert_eq!(config.vision.tflite_model.tensor_width, 400);

            // test saving config
            config.vision.tflite_model.nms_threshold = 66;
            config.paths.try_init_dirs().unwrap();
            config.save();

            let config2 = PrintNannyConfig::new().unwrap();

            assert_eq!(config.vision, config2.vision);

            Ok(())
        });
    }
}

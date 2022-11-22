use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgEnum, PossibleValue};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use git2::{DiffFormat, DiffOptions, Repository};
use glob::glob;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeJsonValue;
use treediff::diff;
use treediff::tools::Recorder;

use super::error::PrintNannySettingsError;
use super::paths::{PrintNannyPaths, DEFAULT_PRINTNANNY_SETTINGS_FILE};
use super::printnanny_api::ApiService;
use super::state::PrintNannyCloudData;
use crate::error::IoError;
use crate::error::ServiceError;
use crate::printer_mgmt;
use printnanny_api_client::models;

// FACTORY_RESET holds the struct field names of PrintNannyCloudConfig
// each member of FACTORY_RESET is written to a separate config fragment under /etc/printnanny/conf.d
// as the name implies, this const is used for performing a reset of any config data modified from defaults
const FACTORY_RESET: [&str; 2] = ["cloud", "systemd_units"];

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

pub trait VersionControlledSettings {
    type SettingsModel: Serialize;
    fn get_git_repo(&self) -> Result<Repository, git2::Error> {
        let settings = PrintNannySettings::new().unwrap();
        Repository::open(self.settings.paths.settings_dir)
    }
    fn get_git_diff_options(&self) -> DiffOptions {
        DiffOptions::new()
            .force_text(true)
            .old_prefix("old")
            .new_prefix("new")
    }
    fn git_diff(&self, repo: &Path) -> Result<String, git2::Error> {
        let repo = self.get_git_repo()?;
        let diffopts = self.get_git_diff_options();
        let mut lines: Vec<String> = vec![];
        repo.diff_index_to_workdir(None, diffopts)
            .print(DiffFormat::Patch, |_delta, _hunk, line| {
                lines.push(str::from_utf8(line.content()).unwrap())
            });
        Ok(lines.join("\n"))
    }
    fn write_settings(&self, content: &str) -> Result<(), IoError> {
        let output = self.get_settings_file()?;
        fs::write(output, content)
    }
    fn git_commit(&self) -> Result<String>;

    fn get_settings_format(&self) -> SettingsFormat;
    fn get_settings_file(&self) -> PathBuf;

    fn git_revert(&self) -> Result<String>;

    fn pre_save(&self) -> Result<()>;
    fn post_save(&self) -> Result<()>;
    fn validate(&self) -> bool;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Deserialize, Serialize)]
pub enum SettingsFormat {
    #[serde(rename = "ini")]
    Ini,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "toml")]
    Toml,
    #[serde(rename = "yaml")]
    Yaml,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

impl SettingsFormat {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        SettingsFormat::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for SettingsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for SettingsFormat {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, clap::ValueEnum, Eq, Deserialize, Serialize, PartialEq)]
pub enum VideoSrcType {
    File,
    Device,
    Uri,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SystemdUnit {
    unit: String,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PrintNannySettings {
    pub paths: PrintNannyPaths,
    pub klipper: printer_mgmt::klipper::KlipperSettings,
    pub mainsail: printer_mgmt::mainsail::MainsailSettings,
    pub moonraker: printer_mgmt::moonraker::MoonrakerSettings,
    pub octoprint: printer_mgmt::octoprint::OctoPrintSettings,
}

impl Default for PrintNannySettings {
    fn default() -> Self {
        Self {
            paths: PrintNannyPaths::default(),
            klipper: printer_mgmt::klipper::KlipperSettings::default(),
            octoprint: printer_mgmt::octoprint::OctoPrintSettings::default(),
            moonraker: printer_mgmt::moonraker::MoonrakerSettings::default(),
            mainsail: printer_mgmt::mainsail::MainsailSettings::default(),
        }
    }
}

impl PrintNannySettings {
    pub fn new() -> Result<Self, PrintNannySettingsError> {
        let figment = Self::figment()?;
        let result = figment.extract()?;
        debug!("Initialized config {:?}", result);
        Ok(result)
    }

    pub fn dashboard_url(&self) -> String {
        let hostname = sys_info::hostname().unwrap_or_else(|_| "printnanny".to_string());
        format!("http://{}.local/", hostname)
    }
    pub fn find_value(key: &str) -> Result<figment::value::Value, PrintNannySettingsError> {
        let figment = Self::figment()?;
        Ok(figment.find_value(key)?)
    }

    pub async fn connect_cloud_account(
        &self,
        base_path: String,
        bearer_access_token: String,
    ) -> Result<(), ServiceError> {
        let state_file = self.paths.state_file();
        let state_lock = self.paths.state_lock();

        let mut state = PrintNannyCloudData::load(&state_file)?;
        state.api.base_path = base_path;
        state.api.bearer_access_token = Some(bearer_access_token);

        state.save(&state_file, &state_lock, true)?;

        let mut api_service = ApiService::new()?;

        // sync data models
        api_service.sync().await?;
        let mut state = PrintNannyCloudData::load(&self.paths.state_file())?;
        let pi_id = state.pi.unwrap().id;
        // download credential and device identity bundled in license.zip
        api_service.pi_download_license(pi_id).await?;
        // mark setup complete
        let req = models::PatchedPiRequest {
            setup_finished: Some(true),
            // None values are skipped by serde serializer
            sbc: None,
            hostname: None,
            fqdn: None,
            favorite: None,
        };
        api_service.pi_partial_update(pi_id, req).await?;
        let pi = api_service.pi_retrieve(pi_id).await?;
        state.pi = Some(pi);
        state.save(&self.paths.state_file(), &self.paths.state_lock(), true)?;
        Ok(())
    }

    // pub async fn sync(&self) -> Result<(), ServiceError> {
    //     let mut service = ApiService::new()?;
    //     service.sync().await
    // }

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
    // 2) PRINTNANNY_SETTINGS .toml. configuration file
    //
    // 3) Glob pattern of .toml and .json configuration file fragments in conf.d folder
    //
    // 4) Defaults (from implement Default)

    pub fn check_file_from_env_var(var: &str) -> Result<(), PrintNannySettingsError> {
        // try reading env var
        match env::var(var) {
            Ok(value) => {
                // check that value exists
                let path = PathBuf::from(value);
                match path.exists() {
                    true => Ok(()),
                    false => Err(PrintNannySettingsError::ConfigFileNotFound { path }),
                }
            }
            Err(_) => {
                warn!(
                    "PRINTNANNY_SETTINGS not set. Initializing from PrintNannyCloudConfig::default()"
                );
                Ok(())
            }
        }
    }

    // load figment fragments from all *.toml and *.json files relative to base_dir
    fn load_confd(base_dir: &Path, figment: Figment) -> Result<Figment, PrintNannySettingsError> {
        let toml_glob = format!("{}/*.toml", &base_dir.display());
        let json_glob = format!("{}/*.json", &base_dir.display());

        let result = Self::read_path_glob::<Json>(&json_glob, figment);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);
        Ok(result)
    }

    pub fn figment() -> Result<Figment, PrintNannySettingsError> {
        // merge file in PRINTNANNY_SETTINGS env var (if set)
        let result = Figment::from(Self { ..Self::default() })
            .merge(Toml::file(Env::var_or(
                "PRINTNANNY_SETTINGS",
                DEFAULT_PRINTNANNY_SETTINGS_FILE,
            )))
            // allow nested environment variables:
            // PRINTNANNY_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_").split("__"));

        // extract paths, to load application state conf.d fragments
        let lib_settings_file: String = result
            .find_value("paths.state_dir")
            .unwrap()
            .deserialize::<String>()
            .unwrap();
        let paths = PrintNannyPaths {
            state_dir: PathBuf::from(lib_settings_file),
            ..PrintNannyPaths::default()
        };

        // merge application state
        let result = Self::load_confd(&paths.lib_confd(), result)?;
        let paths = PrintNannyPaths {
            settings_dir: PathBuf::from(user_settings_file),
            ..PrintNannyPaths::default()
        };

        // merge user-provided config files
        let result = Self::load_confd(&paths.user_confd(), result)?;
        // if PRINTNANNY_SETTINGS env var is set, check file exists and is readable
        Self::check_file_from_env_var("PRINTNANNY_SETTINGS")?;

        // finally, re-merge PRINTNANNY_SETTINGS and PRINTNANNY_ENV so these values take highest precedence
        let result = result
            .merge(Toml::file(Env::var_or(
                "PRINTNANNY_SETTINGS",
                DEFAULT_PRINTNANNY_SETTINGS_FILE,
            )))
            // allow nested environment variables:
            // PRINTNANNY_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_").split("__"));

        info!("Finalized PrintNannyCloudConfig: \n {:?}", result);
        Ok(result)
    }

    pub fn from_toml(f: PathBuf) -> Result<Self, PrintNannySettingsError> {
        let figment = PrintNannySettings::figment()?.merge(Toml::file(f));
        Ok(figment.extract()?)
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

    pub fn try_check_license(&self) -> Result<(), PrintNannySettingsError> {
        let state = PrintNannyCloudData::load(&self.paths.state_file())?;
        match &state.pi {
            Some(_) => Ok(()),
            None => Err(PrintNannySettingsError::SetupIncomplete {
                path: "pi".to_string(),
            }),
        }?;

        match &state.api.bearer_access_token {
            Some(_) => Ok(()),
            None => Err(PrintNannySettingsError::SetupIncomplete {
                path: "api.bearer_access_token".to_string(),
            }),
        }?;

        match self.paths.cloud_nats_creds().exists() {
            true => Ok(()),
            false => Err(PrintNannySettingsError::SetupIncomplete {
                path: self.paths.cloud_nats_creds().display().to_string(),
            }),
        }?;

        match state.pi.as_ref().unwrap().nats_app {
            Some(_) => Ok(()),
            None => Err(PrintNannySettingsError::LicenseMissing {
                path: "pi.nats_app".to_string(),
            }),
        }?;
        Ok(())
    }

    pub fn try_factory_reset(&self) -> Result<(), PrintNannySettingsError> {
        // for each key/value pair in FACTORY_RESET, remove file
        for key in FACTORY_RESET.iter() {
            let filename = format!("{}.json", key);
            let filename = self.paths.lib_confd().join(filename);
            fs::remove_file(&filename)?;
            info!("Removed {} data {:?}", key, filename);
        }
        Ok(())
    }

    // Save settings to PRINTNANNY_SETTINGS (default: /var/lib/printnanny/PrintNannySettings.toml)
    pub fn try_save(&self) -> Result<(), PrintNannySettingsError> {
        let settings_file = self.paths.settings_file();
        let settings_data = toml::ser::to_string_pretty(self)?;
        fs::write(&settings_file, &settings_data)?;
        Ok(())
    }
    // Save settings to PRINTNANNY_SETTINGS (default: /var/lib/printnanny/PrintNannySettings.toml)
    pub fn save(&self) {
        self.try_save().expect("Failed to save PrintNannySettings");
    }

    // Save ::Default() to output file
    pub fn try_init(
        &self,
        filename: &str,
        format: &SettingsFormat,
    ) -> Result<(), PrintNannySettingsError> {
        let content: String = match format {
            SettingsFormat::Json => serde_json::to_string_pretty(self)?,
            SettingsFormat::Toml => toml::ser::to_string_pretty(self)?,
        };
        fs::write(&filename, content)?;
        Ok(())
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

impl Provider for PrintNannySettings {
    fn metadata(&self) -> Metadata {
        Metadata::named("PrintNannySettings")
    }

    fn data(&self) -> figment::error::Result<Map<Profile, Dict>> {
        let map: Map<Profile, Dict> = Serialized::defaults(self).data()?;
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::PRINTNANNY_SETTINGS_FILENAME;

    #[test_log::test]
    fn test_config_file_not_found() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let result = PrintNannySettings::figment();
            assert!(result.is_err());
            Ok(())
        });
    }

    #[test_log::test]
    fn test_nested_env_var() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_SETTINGS_FILENAME,
                r#"
                [paths]
                settings_dir = "/this/etc/path/gets/overridden"
                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let expected = PathBuf::from("testing");
            jail.set_env("PRINTNANNY_PATHS__SETTINGS_DIR", &expected.display());
            let figment = PrintNannySettings::figment().unwrap();
            let config: PrintNannySettings = figment.extract()?;
            assert_eq!(config.paths.settings_dir, expected);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_paths() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_SETTINGS_FILENAME,
                r#"
                [paths]
                settings_dir = "/opt/printnanny/"
                state_dir = "/var/lib/custom"
        
                
                [api]
                base_path = "https://print-nanny.com"
                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let figment = PrintNannySettings::figment().unwrap();
            let config: PrintNannySettings = figment.extract()?;
            assert_eq!(config.paths.data(), PathBuf::from("/var/lib/custom/data"));
            assert_eq!(config.paths.user_confd(), PathBuf::from("/opt/printnanny/"));

            Ok(())
        });
    }
    #[test_log::test]
    fn test_env_merged() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                PRINTNANNY_SETTINGS_FILENAME,
                r#"
                [paths]
                install = "/opt/printnanny/default"
                data = "/opt/printnanny/default/data"

                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let settings = PrintNannySettings::new().unwrap();
            assert_eq!(
                settings.octoprint.enabled,
                printer_mgmt::octoprint::OctoPrintSettings::default().enabled,
            );
            jail.set_env("PRINTNANNY_OCTOPRINT__ENABLED", "false");
            let figment = PrintNannySettings::figment().unwrap();
            let settings: PrintNannySettings = figment.extract()?;
            assert_eq!(settings.octoprint.enabled, false);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_conf_values() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
                [paths]
                settings_dir = ".tmp/"
                
                [octoprint]
                enabled = false
                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", "Local.toml");

            let figment = PrintNannySettings::figment().unwrap();
            let settings: PrintNannySettings = figment.extract()?;

            assert_eq!(settings.paths.settings_dir, PathBuf::from(".tmp/"));
            assert_eq!(settings.octoprint.enabled, false);

            Ok(())
        });
    }

    #[test_log::test]
    fn test_save() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();
            jail.create_file(
                "Local.toml",
                &format!(
                    r#"
                profile = "local"

                [paths]
                state_dir = "{}"

                [octoprint]
                enabled = false
                "#,
                    output
                ),
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", "Local.toml");

            let figment = PrintNannySettings::figment().unwrap();
            let mut settings: PrintNannySettings = figment.extract()?;
            fs::create_dir(settings.paths.lib_confd()).unwrap();

            settings.octoprint.enabled = true;
            settings.save();
            let figment = PrintNannySettings::figment().unwrap();
            let settings: PrintNannySettings = figment.extract()?;
            assert_eq!(settings.octoprint.enabled, true);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_find_value() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();
            let expected: Option<String> = Some(format!("{output}/printnanny.d"));

            jail.create_file(
                "Local.toml",
                &format!(
                    r#"
                [paths]
                settings_dir = "{output}/printnanny.d"
                log_dir = "{output}/log"

                [octoprint]
                enabled = false
                "#,
                    output = &output
                ),
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", "Local.toml");

            let value: Option<String> = PrintNannySettings::find_value("paths.settings_dir")
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
                PRINTNANNY_SETTINGS_FILENAME,
                r#"
                [octoprint]
                enabled = false
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
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            jail.set_env(
                "PRINTNANNY_PATHS__OS_RELEASE",
                format!("{:?}", jail.directory().join("os-release")),
            );

            let config = PrintNannySettings::new().unwrap();
            let os_release = config.paths.load_os_release().unwrap();
            assert_eq!("2022-06-18T18:46:49Z".to_string(), os_release.build_id);
            Ok(())
        });
    }

    #[test_log::test]
    fn test_user_provided_toml_file() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();

            let filename = "custom.toml";

            jail.create_file(
                filename,
                &format!(
                    r#"
                profile = "local"
                [paths]
                settings_dir = "{output}/printnanny.d"
                log_dir = "{output}/log"
                "#,
                    output = output
                ),
            )?;

            let config =
                PrintNannySettings::from_toml(PathBuf::from(output).join(filename)).unwrap();
            assert_eq!(
                config.paths.settings_dir,
                PathBuf::from(format!("{}/printnanny.d", output))
            );

            Ok(())
        });
    }
}

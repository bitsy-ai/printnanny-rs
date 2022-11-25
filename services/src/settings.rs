use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgEnum, PossibleValue};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use git2::Repository;
use glob::glob;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use super::error::PrintNannySettingsError;

use super::paths::{PrintNannyPaths, DEFAULT_PRINTNANNY_SETTINGS_FILE};
use super::printnanny_api::ApiService;
use super::state::PrintNannyCloudData;
use crate::error::ServiceError;
use crate::printer_mgmt;
use crate::vcs::VersionControlledSettings;
use printnanny_api_client::models;

// FACTORY_RESET holds the struct field names of PrintNannyCloudConfig
// each member of FACTORY_RESET is written to a separate config fragment under /etc/printnanny/conf.d
// as the name implies, this const is used for performing a reset of any config data modified from defaults
const FACTORY_RESET: [&str; 2] = ["cloud", "systemd_units"];

const DEFAULT_PRINTNANNY_SETTINGS_GIT_REMOTE: &str =
    "https://github.com/bitsy-ai/printnanny-settings.git";

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Deserialize, Serialize)]
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
    pub git_remote: String,
}

impl Default for PrintNannySettings {
    fn default() -> Self {
        Self {
            paths: PrintNannyPaths::default(),
            klipper: printer_mgmt::klipper::KlipperSettings::default(),
            octoprint: printer_mgmt::octoprint::OctoPrintSettings::default(),
            moonraker: printer_mgmt::moonraker::MoonrakerSettings::default(),
            mainsail: printer_mgmt::mainsail::MainsailSettings::default(),
            git_remote: DEFAULT_PRINTNANNY_SETTINGS_GIT_REMOTE.into(),
        }
    }
}

impl PrintNannySettings {
    pub fn new() -> Result<Self, PrintNannySettingsError> {
        let figment = Self::figment()?;
        let mut result: PrintNannySettings = figment.extract()?;

        result.octoprint =
            printer_mgmt::octoprint::OctoPrintSettings::from_dir(&result.paths.settings_dir);
        debug!("Initialized config {:?}", result);

        Ok(result)
    }

    pub fn git_clone(&self) -> Result<Repository, git2::Error> {
        let repo = Repository::clone(&self.git_remote, &self.paths.settings_dir)?;
        Ok(repo)
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
            // PRINTNANNY_SETTINGS_KEY__SUBKEY
            .merge(Env::prefixed("PRINTNANNY_SETTINGS_").split("__"));

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
            .merge(Env::prefixed("PRINTNANNY_SETTINGS_").split("__"));

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
            _ => unimplemented!("try_init is not implemented for format: {}", format),
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

pub mod jail {
    use std::collections::HashMap;
    use std::ffi::{OsStr, OsString};
    use std::fmt::Display;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::path::{Path, PathBuf};

    use parking_lot::Mutex;
    use tempfile::TempDir;

    use figment::error::Result;

    /// Based on: https://github.com/SergioBenitez/Figment/blob/master/src/jail.rs
    /// with Clone implementation
    /// environment variables before entering this? Will they mess with
    // anything else?
    /// A "sandboxed" environment with isolated env and file system namespace.
    ///
    /// `Jail` creates a pseudo-sandboxed (not _actually_ sandboxed) environment for
    /// testing configurations. Specifically, `Jail`:
    ///
    ///   * Synchronizes all calls to [`Jail::expect_with()`] and
    ///     [`Jail::try_with()`] to prevent environment variables races.
    ///   * Switches into a fresh temporary directory ([`Jail::directory()`]) where
    ///     files can be created with [`Jail::create_file()`].
    ///   * Keeps track of environment variables created with [`Jail::set_env()`]
    ///     and clears them when the `Jail` exits.
    ///   * Deletes the temporary directory and all of its contents when exiting.
    ///
    /// Additionally, because `Jail` expects functions that return a [`Result`],
    /// the `?` operator can be used liberally in a jail:
    ///
    /// ```rust
    /// use figment::{Figment, Jail, providers::{Format, Toml, Env}};
    /// # #[derive(serde::Deserialize)]
    /// # struct Config {
    /// #     name: String,
    /// #     authors: Vec<String>,
    /// #     publish: bool
    /// # }
    ///
    /// figment::Jail::expect_with(|jail| {
    ///     jail.create_file("Cargo.toml", r#"
    ///       name = "test"
    ///       authors = ["bob"]
    ///       publish = false
    ///     "#)?;
    ///
    ///     jail.set_env("CARGO_NAME", "env-test");
    ///
    ///     let config: Config = Figment::new()
    ///         .merge(Toml::file("Cargo.toml"))
    ///         .merge(Env::prefixed("CARGO_"))
    ///         .extract()?;
    ///
    ///     Ok(())
    /// });
    /// ```
    #[cfg_attr(nightly, doc(cfg(feature = "test")))]
    #[derive(Debug)]
    pub struct Jail {
        _directory: TempDir,
        canonical_dir: PathBuf,
        saved_env_vars: HashMap<OsString, Option<OsString>>,
        saved_cwd: PathBuf,
    }

    fn as_string<S: Display>(s: S) -> String {
        s.to_string()
    }

    static LOCK: Mutex<()> = parking_lot::const_mutex(());

    impl Jail {
        /// Creates a new jail that calls `f`, passing itself to `f`.
        ///
        /// # Panics
        ///
        /// Panics if `f` panics or if [`Jail::try_with(f)`](Jail::try_with) returns
        /// an `Err`; prints the error message.
        ///
        /// # Example
        ///
        /// ```rust
        /// figment::Jail::expect_with(|jail| {
        ///     /* in the jail */
        ///
        ///     Ok(())
        /// });
        /// ```
        #[track_caller]
        pub fn expect_with<F: FnOnce(&mut Jail) -> Result<()>>(f: F) {
            if let Err(e) = Jail::try_with(f) {
                panic!("jail failed: {}", e)
            }
        }

        /// Creates a new jail that calls `f`, passing itself to `f`. Returns the
        /// result from `f` if `f` does not panic.
        ///
        /// # Panics
        ///
        /// Panics if `f` panics.
        ///
        /// # Example
        ///
        /// ```rust
        /// let result = figment::Jail::try_with(|jail| {
        ///     /* in the jail */
        ///
        ///     Ok(())
        /// });
        /// ```
        #[track_caller]
        pub fn try_with<F: FnOnce(&mut Jail) -> Result<()>>(f: F) -> Result<()> {
            let _lock = LOCK.lock();
            let directory = TempDir::new().map_err(as_string)?;
            let mut jail = Jail {
                canonical_dir: directory.path().canonicalize().map_err(as_string)?,
                _directory: directory,
                saved_cwd: std::env::current_dir().map_err(as_string)?,
                saved_env_vars: HashMap::new(),
            };

            std::env::set_current_dir(jail.directory()).map_err(as_string)?;
            f(&mut jail)
        }

        pub fn new() -> Result<Jail> {
            let _lock = LOCK.lock();
            let directory = TempDir::new().map_err(as_string)?;
            let mut jail = Jail {
                canonical_dir: directory.path().canonicalize().map_err(as_string)?,
                _directory: directory,
                saved_cwd: std::env::current_dir().map_err(as_string)?,
                saved_env_vars: HashMap::new(),
            };

            std::env::set_current_dir(jail.directory()).map_err(as_string)?;
            Ok(jail)
        }

        /// Returns the directory the jail has switched into. The contents of this
        /// directory will be cleared when `Jail` is dropped.
        ///
        /// # Example
        ///
        /// ```rust
        /// figment::Jail::expect_with(|jail| {
        ///     let tmp_directory = jail.directory();
        ///
        ///     Ok(())
        /// });
        /// ```
        pub fn directory(&self) -> &Path {
            &self.canonical_dir
        }

        /// Creates a file with contents `contents` in the jail's directory. The
        /// file will be deleted with the jail is dropped.
        ///
        /// # Example
        ///
        /// ```rust
        /// figment::Jail::expect_with(|jail| {
        ///     jail.create_file("MyConfig.json", "contents...");
        ///     Ok(())
        /// });
        /// ```
        pub fn create_file<P: AsRef<Path>>(&self, path: P, contents: &str) -> Result<File> {
            let path = path.as_ref();
            if !path.is_relative() {
                return Err("Jail::create_file(): file path is absolute"
                    .to_string()
                    .into());
            }

            let file = File::create(self.directory().join(path)).map_err(as_string)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(contents.as_bytes()).map_err(as_string)?;
            Ok(writer.into_inner().map_err(as_string)?)
        }

        /// Set the environment variable `k` to value `v`. The variable will be
        /// removed when the jail is dropped.
        ///
        /// # Example
        ///
        /// ```rust
        /// const VAR_NAME: &str = "my-very-special-figment-var";
        ///
        /// assert!(std::env::var(VAR_NAME).is_err());
        ///
        /// figment::Jail::expect_with(|jail| {
        ///     jail.set_env(VAR_NAME, "value");
        ///     assert!(std::env::var(VAR_NAME).is_ok());
        ///     Ok(())
        /// });
        ///
        /// assert!(std::env::var(VAR_NAME).is_err());
        /// ```
        pub fn set_env<K: AsRef<str>, V: Display>(&mut self, k: K, v: V) {
            let key = k.as_ref();
            if !self.saved_env_vars.contains_key(OsStr::new(key)) {
                self.saved_env_vars
                    .insert(key.into(), std::env::var_os(key));
            }

            std::env::set_var(key, v.to_string());
        }
    }

    impl Drop for Jail {
        fn drop(&mut self) {
            for (key, value) in self.saved_env_vars.iter() {
                match value {
                    Some(val) => std::env::set_var(key, val),
                    None => std::env::remove_var(key),
                }
            }

            let _ = std::env::set_current_dir(&self.saved_cwd);
        }
    }
}

#[cfg(test)]
pub mod tests {
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
            jail.set_env(
                "PRINTNANNY_SETTINGS_PATHS__SETTINGS_DIR",
                &expected.display(),
            );
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
            jail.set_env("PRINTNANNY_SETTINGS_OCTOPRINT__ENABLED", "false");
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
                "PRINTNANNY_SETTINGS_PATHS__OS_RELEASE",
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

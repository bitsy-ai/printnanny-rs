use std::env;
// use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use figment::providers::{Env, Format, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Figment, Metadata, Profile, Provider};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use tokio::fs;

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use crate::cam::VideoStreamSettings;
use crate::error::{PrintNannySettingsError, VersionControlledSettingsError};
use crate::klipper::{KlipperSettings, DEFAULT_KLIPPER_SETTINGS_FILE};
use crate::moonraker::{MoonrakerSettings, DEFAULT_MOONRAKER_SETTINGS_FILE};
use crate::octoprint::{OctoPrintSettings, DEFAULT_OCTOPRINT_SETTINGS_FILE};
use crate::paths::{PrintNannyPaths, DEFAULT_PRINTNANNY_SETTINGS_FILE};
use crate::vcs::VersionControlledSettings;
use crate::SettingsFormat;

pub const DEFAULT_PRINTNANNY_SETTINGS_DIR: &str = "/home/printnanny/.config/printnanny/vcs";

const DEFAULT_PRINTNANNY_SETTINGS_GIT_REMOTE: &str =
    "https://github.com/bitsy-ai/printnanny-settings.git";
const DEFAULT_PRINTNANNY_SETTINGS_GIT_EMAIL: &str = "robots@printnanny.ai";
const DEFAULT_PRINTNANNY_SETTINGS_GIT_NAME: &str = "PrintNanny";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrintNannyApiConfig {
    pub api_base_path: String,
    pub api_bearer_access_token: Option<String>,
}

impl Default for PrintNannyApiConfig {
    fn default() -> Self {
        // default to unauthenticated api config, until user connects their PrintNanny Cloud account
        Self {
            api_base_path: "https://printnanny.ai".into(),
            api_bearer_access_token: None,
        }
    }
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
pub struct GitSettings {
    pub path: PathBuf, // local git repo used to commit/revert changes to user-supplied config
    pub remote: String,
    pub email: String,
    pub name: String,
    pub default_branch: String,
}

impl Default for GitSettings {
    fn default() -> Self {
        Self {
            path: DEFAULT_PRINTNANNY_SETTINGS_DIR.into(),
            remote: DEFAULT_PRINTNANNY_SETTINGS_GIT_REMOTE.into(),
            email: DEFAULT_PRINTNANNY_SETTINGS_GIT_EMAIL.into(),
            name: DEFAULT_PRINTNANNY_SETTINGS_GIT_NAME.into(),
            default_branch: "main".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PrintNannySettings {
    pub video_stream: VideoStreamSettings,
    pub cloud: PrintNannyApiConfig,
    pub git: GitSettings,
    pub paths: PrintNannyPaths,
}

impl Default for PrintNannySettings {
    fn default() -> Self {
        let git = GitSettings::default();
        let video_stream = VideoStreamSettings::default();

        Self {
            cloud: PrintNannyApiConfig::default(),
            paths: PrintNannyPaths::default(),
            git,
            video_stream,
        }
    }
}

impl PrintNannySettings {
    pub async fn new() -> Result<Self, PrintNannySettingsError> {
        let figment = Self::figment().await?;
        let result: PrintNannySettings = figment.extract()?;
        debug!("Initialized config {:?}", result);

        Ok(result)
    }

    pub fn to_octoprint_settings(&self) -> OctoPrintSettings {
        let git_settings = self.git.clone();
        let settings_file = self.git.path.join(DEFAULT_OCTOPRINT_SETTINGS_FILE);

        OctoPrintSettings {
            git_settings,
            settings_file,
            ..OctoPrintSettings::default()
        }
    }

    pub fn to_moonraker_settings(&self) -> MoonrakerSettings {
        let git_settings = self.git.clone();
        let settings_file = self.git.path.join(DEFAULT_MOONRAKER_SETTINGS_FILE);
        MoonrakerSettings {
            git_settings,
            settings_file,
            ..MoonrakerSettings::default()
        }
    }

    pub fn to_klipper_settings(&self) -> KlipperSettings {
        let git_settings = self.git.clone();
        let settings_file = self.git.path.join(DEFAULT_KLIPPER_SETTINGS_FILE);

        KlipperSettings {
            git_settings,
            settings_file,
            ..KlipperSettings::default()
        }
    }

    pub fn dashboard_url(&self) -> String {
        let hostname = sys_info::hostname().unwrap_or_else(|_| "printnanny".to_string());
        format!("http://{}.local/", hostname)
    }
    pub async fn find_value(key: &str) -> Result<figment::value::Value, PrintNannySettingsError> {
        let figment = Self::figment().await?;
        Ok(figment.find_value(key)?)
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
                debug!(
                    "PRINTNANNY_SETTINGS not set. Initializing from PrintNannyCloudConfig::default()"
                );
                Ok(())
            }
        }
    }

    pub async fn figment() -> Result<Figment, PrintNannySettingsError> {
        // if PRINTNANNY_SETTINGS env var is set, check file exists and is readable
        Self::check_file_from_env_var("PRINTNANNY_SETTINGS")?;
        // merge file in PRINTNANNY_SETTINGS env var (if set)
        let file_path_str = Env::var_or("PRINTNANNY_SETTINGS", DEFAULT_PRINTNANNY_SETTINGS_FILE);
        let file_path = PathBuf::from(&file_path_str);
        let result = match file_path.exists() {
            true => {
                let file_contents = fs::read_to_string(file_path).await?;
                Figment::from(Self { ..Self::default() })
                    .merge(Toml::string(&file_contents))
                    // allow nested environment variables:
                    // PRINTNANNY_SETTINGS_KEY__SUBKEY
                    .merge(Env::prefixed("PRINTNANNY_SETTINGS_").split("__"))
            }
            false => {
                Figment::from(Self { ..Self::default() })
                    // allow nested environment variables:
                    // PRINTNANNY_SETTINGS_KEY__SUBKEY
                    .merge(Env::prefixed("PRINTNANNY_SETTINGS_").split("__"))
            }
        };
        debug!("Finalized PrintNannySettings: \n {:?}", result);
        Ok(result)
    }

    pub async fn from_toml(f: PathBuf) -> Result<Self, PrintNannySettingsError> {
        let file_contents = fs::read_to_string(f).await?;
        let figment = PrintNannySettings::figment()
            .await?
            .merge(Toml::string(&file_contents));
        Ok(figment.extract()?)
    }

    pub fn to_toml_string(&self) -> Result<String, PrintNannySettingsError> {
        let result = toml::ser::to_string_pretty(self)?;
        Ok(result)
    }

    pub fn try_factory_reset(&self) -> Result<(), PrintNannySettingsError> {
        // for each key/value pair in FACTORY_RESET, remove file
        todo!()
    }

    // Save settings to PRINTNANNY_SETTINGS
    pub async fn try_save(&self) -> Result<(), PrintNannySettingsError> {
        let settings_file = self.paths.settings_file();
        let settings_data = toml::ser::to_string_pretty(self)?;
        fs::write(settings_file, settings_data).await?;
        Ok(())
    }
    // Save settings to PRINTNANNY_SETTINGS
    pub async fn save(&self) {
        self.try_save()
            .await
            .expect("Failed to save PrintNannySettings");
    }

    // Save ::Default() to output file
    pub async fn try_init(
        &self,
        filename: &str,
        format: &SettingsFormat,
    ) -> Result<(), PrintNannySettingsError> {
        let content: String = match format {
            SettingsFormat::Json => serde_json::to_string_pretty(self)?,
            SettingsFormat::Toml => toml::ser::to_string_pretty(self)?,
            _ => unimplemented!("try_init is not implemented for format: {}", format),
        };
        fs::write(filename, content).await?;
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

    pub async fn detect_hls_http_enabled(&self) -> Result<bool, zbus::Error> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let unit_path = proxy
            .get_unit_file_state("octoprint.service".into())
            .await?;

        let result = &unit_path == "enabled";
        Ok(result)
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

#[async_trait]
impl VersionControlledSettings for PrintNannySettings {
    type SettingsModel = PrintNannySettings;
    fn from_dir(_settings_dir: &Path) -> Self {
        todo!()
    }
    fn get_settings_format(&self) -> SettingsFormat {
        SettingsFormat::Toml
    }
    fn get_settings_file(&self) -> PathBuf {
        self.paths.settings_file()
    }
    async fn pre_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running PrintNannySettings pre_save hook");
        Ok(())
    }

    async fn post_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running PrintNannySettings post_save hook");
        Ok(())
    }
    fn validate(&self) -> Result<(), VersionControlledSettingsError> {
        todo!("PrintNannySettings validate hook is not yet implemented");
    }

    fn get_git_repo_path(&self) -> &Path {
        &self.git.path
    }

    fn get_git_remote(&self) -> &str {
        &self.git.remote
    }

    fn get_git_settings(&self) -> &GitSettings {
        &self.git
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::PRINTNANNY_SETTINGS_FILENAME;
    use tokio::runtime::Runtime;

    #[test_log::test]
    fn test_config_file_not_found() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let result = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::figment());
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
                log_dir = "/this/etc/path/gets/overridden"
                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let expected = PathBuf::from("testing");
            jail.set_env("PRINTNANNY_SETTINGS_PATHS__LOG_DIR", &expected.display());
            let figment = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::figment())
                .unwrap();
            let config: PrintNannySettings = figment.extract()?;
            assert_eq!(config.paths.log_dir, expected);
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
                state_dir = "/var/lib/custom"

                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", PRINTNANNY_SETTINGS_FILENAME);
            let figment = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::figment())
                .unwrap();
            let config: PrintNannySettings = figment.extract()?;
            assert_eq!(config.paths.data(), PathBuf::from("/var/lib/custom/data"));
            assert_eq!(config.paths.state_dir, PathBuf::from("/var/lib/custom/"));

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
            let settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::new())
                .unwrap();
            assert_eq!(settings.git.remote, GitSettings::default().remote);
            jail.set_env("PRINTNANNY_SETTINGS_GIT__REMOTE", "foo.git");
            let settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::new())
                .unwrap();
            assert_eq!(settings.git.remote, "foo.git");
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
                log_dir = ".tmp/"
                
                "#,
            )?;
            jail.set_env("PRINTNANNY_SETTINGS", "Local.toml");

            let settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::new())
                .unwrap();

            assert_eq!(settings.paths.log_dir, PathBuf::from(".tmp/"));

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

            let mut settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::new())
                .unwrap();

            settings.paths.state_dir = PathBuf::from("foo");
            let runtime = Runtime::new().unwrap();
            runtime.block_on(settings.save());
            let settings = runtime.block_on(PrintNannySettings::new()).unwrap();
            assert_eq!(settings.paths.state_dir, PathBuf::from("foo"));
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

            let value = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::find_value("paths.settings_dir"))
                .unwrap()
                .into_string();

            assert_eq!(value, expected);
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
                log_dir = "{output}/log"

                [git]
                path = "{output}/printnanny.d"
                "#,
                    output = output
                ),
            )?;

            let settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::from_toml(
                    PathBuf::from(output).join(filename),
                ))
                .unwrap();

            assert_eq!(
                settings.git.path,
                PathBuf::from(format!("{}/printnanny.d", output))
            );

            assert_eq!(
                settings.paths.log_dir,
                PathBuf::from(format!("{}/log", output))
            );

            Ok(())
        });
    }

    #[test_log::test]
    fn test_cam_settings() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();

            let filename = "custom.toml";

            jail.create_file(
                filename,
                r#"
                [video_stream.detection]
                tensor_framerate = 1
                "#,
            )?;

            let settings = Runtime::new()
                .unwrap()
                .block_on(PrintNannySettings::from_toml(
                    PathBuf::from(output).join(filename),
                ))
                .unwrap();
            assert_eq!(settings.video_stream.detection.tensor_framerate, 1);

            Ok(())
        });
    }
}

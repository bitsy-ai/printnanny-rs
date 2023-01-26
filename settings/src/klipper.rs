use std::path::{Path, PathBuf};

use async_trait::async_trait;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use crate::error::VersionControlledSettingsError;
use crate::printnanny::GitSettings;
use crate::vcs::{VersionControlledSettings, DEFAULT_VCS_SETTINGS_DIR};
use crate::SettingsFormat;

pub const KLIPPER_INSTALL_DIR: &str = "/home/printnanny/.klipper";
pub const KLIPPER_VENV: &str = "/home/printnanny/klipper-venv";
pub const DEFAULT_KLIPPER_SETTINGS_FILE: &str = "/klipper/printer.cfg";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KlipperSettings {
    pub enabled: bool,
    pub install_dir: PathBuf,
    pub settings_file: PathBuf,
    pub settings_format: SettingsFormat,
    pub venv: PathBuf,
    pub git_settings: GitSettings,
}

impl Default for KlipperSettings {
    fn default() -> Self {
        let install_dir: PathBuf = KLIPPER_INSTALL_DIR.into();
        let settings_file =
            PathBuf::from(DEFAULT_VCS_SETTINGS_DIR).join(DEFAULT_KLIPPER_SETTINGS_FILE);

        let git_settings = GitSettings::default();

        Self {
            settings_file,
            install_dir,
            enabled: false,
            venv: KLIPPER_VENV.into(),
            settings_format: SettingsFormat::Ini,
            git_settings,
        }
    }
}

#[async_trait]
impl VersionControlledSettings for KlipperSettings {
    type SettingsModel = KlipperSettings;

    fn from_dir(settings_dir: &Path) -> Self {
        let settings_file = settings_dir.join("klipper/printer.cfg");
        Self {
            settings_file,
            ..Self::default()
        }
    }
    fn get_settings_format(&self) -> SettingsFormat {
        self.settings_format
    }
    fn get_settings_file(&self) -> PathBuf {
        self.settings_file.clone()
    }

    fn get_git_repo_path(&self) -> &Path {
        &self.git_settings.path
    }

    fn get_git_remote(&self) -> &str {
        &self.git_settings.remote
    }

    fn get_git_settings(&self) -> &GitSettings {
        &self.git_settings
    }

    async fn pre_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running KlipperSettings pre_save hook");
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .stop_unit("klipper.service".to_string(), "replace".to_string())
            .await?;
        info!("Stopped klipper.service, job: {:?}", job);
        Ok(())
    }

    async fn post_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running KlipperSettings post_save hook");
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .start_unit("klipper.service".into(), "replace".into())
            .await?;
        info!("Started klipper.service, job: {:?}", job);

        Ok(())
    }
    fn validate(&self) -> Result<(), VersionControlledSettingsError> {
        todo!("KlipperSettings validate hook is not yet implemented");
    }
}

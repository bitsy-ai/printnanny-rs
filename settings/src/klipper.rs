use std::path::{Path, PathBuf};

use async_trait::async_trait;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use crate::error::VersionControlledSettingsError;
use crate::vcs::VersionControlledSettings;
use crate::SettingsFormat;

pub const KLIPPER_INSTALL_DIR: &str = "/home/printnanny/.klipper";
pub const KLIPPER_VENV: &str = "/home/printnanny/klipper-venv";
pub const KLIPPER_SETTINGS_FILE: &str =
    "/home/printnanny/.config/printnanny/vcs/klipper/printer.cfg";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KlipperSettings {
    pub enabled: bool,
    pub install_dir: PathBuf,
    pub settings_file: PathBuf,
    pub settings_format: SettingsFormat,
    pub venv: PathBuf,
}

impl Default for KlipperSettings {
    fn default() -> Self {
        let install_dir: PathBuf = KLIPPER_INSTALL_DIR.into();
        let settings_file = KLIPPER_SETTINGS_FILE.into();
        Self {
            settings_file,
            install_dir,
            enabled: false,
            venv: KLIPPER_VENV.into(),
            settings_format: SettingsFormat::Ini,
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
    fn get_settings_format() -> SettingsFormat {
        self.settings_format
    }
    fn get_settings_file(&self) -> PathBuf {
        self.settings_file.clone()
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

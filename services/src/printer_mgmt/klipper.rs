use std::path::PathBuf;

use super::moonraker::MOONRAKER_VENV_PATH;
use serde::{Deserialize, Serialize};

pub const KLIPPER_BASE_PATH: &str = "/var/lib/klipper";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KlipperSettings {
    pub enabled: bool,
    pub install_path: PathBuf,
    pub config_path: PathBuf,
    pub venv_path: PathBuf,
}

impl Default for KlipperSettings {
    fn default() -> Self {
        let install_path: PathBuf = KLIPPER_BASE_PATH.into();
        let config_path = install_path.join("printer.cfg");
        Self {
            config_path,
            install_path,
            enabled: false,
            venv_path: MOONRAKER_VENV_PATH.into(), // klipper shares moonraker virtual environment
        }
    }
}

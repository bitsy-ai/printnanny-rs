use std::path::PathBuf;

use super::moonraker::MOONRAKER_VENV;
use serde::{Deserialize, Serialize};

pub const KLIPPER_INSTALL_DIR: &str = "/var/lib/klipper";
// /var/lib/printnanny/settings contains a local git repo used to commit/revert changes to settings
pub const KLIPPER_SETTINGS_FILE: &str = "/var/lib/printnanny/settings/klipper/printer.cfg";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KlipperSettings {
    pub enabled: bool,
    pub install_dir: PathBuf,
    pub settings_file: PathBuf,
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
            venv: MOONRAKER_VENV.into(), // klipper shares moonraker virtual environment
        }
    }
}

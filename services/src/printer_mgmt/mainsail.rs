use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const MAINSAIL_BASE_PATH: &str = "/var/www/mainsail";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainsailSettings {
    pub enabled: bool,
    pub install_path: PathBuf,
}

impl Default for MainsailSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            install_path: MAINSAIL_BASE_PATH.into(),
        }
    }
}

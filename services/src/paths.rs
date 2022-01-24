use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintNannyPath {
    pub backups: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub keys: PathBuf,
    // this struct
    pub paths_json: PathBuf,
    // api config
    pub api_config_json: PathBuf,
}

impl PrintNannyPath {
    pub fn new(base_str: &str) -> Self {
        let base = PathBuf::from(base_str);
        let backups = base.join("backups");
        let data = base.join("data");
        let keys = base.join("keys");

        let api_config_json = data.join("api_config.toml");
        let paths_json = data.join("paths.toml");

        Self {
            api_config_json,
            backups,
            base,
            data,
            keys,
            paths_json,
        }
    }
}

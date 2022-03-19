use serde::{Deserialize, Serialize};
use std::fs;
const IMAGE_VERSION_FILE: &str = "/boot/image_version.txt";
const OS_VERSION_FILE: &str = "/etc/printnanny/os-release";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub cli_version: String,
    pub image_version: String,
    pub os_version: String,
}

impl VersionInfo {
    pub fn new() -> Self {
        let image_version = fs::read_to_string(IMAGE_VERSION_FILE).unwrap_or("dev".to_string());
        let os_version = fs::read_to_string(OS_VERSION_FILE)
            .unwrap_or("VERSION=dev".to_string())
            .split("=")
            .last()
            .unwrap_or("dev")
            .to_string();
        Self {
            image_version,
            os_version,
            cli_version: CLI_VERSION.to_string(),
        }
    }
}

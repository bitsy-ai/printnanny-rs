use std::fs;
use std::{io::Write, path::Path};

use file_lock::{FileLock, FileOptions};
use serde::{Deserialize, Serialize};

use crate::settings::PrintNannySettings;

use super::error::PrintNannyCloudDataError;
use super::printnanny_api::PrintNannyApiConfig;
use printnanny_api_client::models;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyCloudData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pi: Option<models::Pi>,
    pub api: PrintNannyApiConfig,
}

impl Default for PrintNannyCloudData {
    fn default() -> Self {
        // default to unauthenticated api config, until api creds are unpacked from seed archive
        let api = PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
        };
        PrintNannyCloudData { api, pi: None }
    }
}

impl PrintNannyCloudData {
    pub fn new() -> Result<PrintNannyCloudData, PrintNannyCloudDataError> {
        let settings = PrintNannySettings::new().unwrap();
        let result = Self::load(&settings.paths.state_file())?;
        Ok(result)
    }

    pub fn save(
        &self,
        state_file: &Path,
        state_lock: &Path,
        is_blocking: bool,
    ) -> Result<(), PrintNannyCloudDataError> {
        let options = FileOptions::new().write(true).create(true).append(true);
        let mut filelock = match FileLock::lock(state_lock, is_blocking, options) {
            Ok(lock) => lock,
            Err(err) => panic!("Error getting write lock: {}", err),
        };
        let data = toml::ser::to_vec(self)?;

        match filelock.file.write_all(&data) {
            Ok(()) => Ok(()),
            Err(e) => Err(PrintNannyCloudDataError::WriteIOError {
                path: state_file.display().to_string(),
                error: e,
            }),
        }
    }

    pub fn try_check_cloud_data(&self) -> Result<(), PrintNannyCloudDataError> {
        let settings = PrintNannySettings::new().unwrap();
        let state = PrintNannyCloudData::load(&settings.paths.state_file())?;
        match &state.pi {
            Some(_) => Ok(()),
            None => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: "pi".to_string(),
            }),
        }?;

        match &state.api.bearer_access_token {
            Some(_) => Ok(()),
            None => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: "api.bearer_access_token".to_string(),
            }),
        }?;

        match settings.paths.cloud_nats_creds().exists() {
            true => Ok(()),
            false => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: settings.paths.cloud_nats_creds().display().to_string(),
            }),
        }?;

        match state.pi.as_ref().unwrap().nats_app {
            Some(_) => Ok(()),
            None => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: "pi.nats_app".to_string(),
            }),
        }?;
        Ok(())
    }

    pub fn load(state_file: &Path) -> Result<PrintNannyCloudData, PrintNannyCloudDataError> {
        let state_str = match fs::read_to_string(state_file) {
            Ok(d) => Ok(d),
            Err(e) => Err(PrintNannyCloudDataError::ReadIOError {
                path: state_file.display().to_string(),
                error: e,
            }),
        }?;
        let state: PrintNannyCloudData = toml::de::from_str(&state_str)?;
        Ok(state)
    }
}

use std::fs;
use std::path::Path;

use log::info;
use serde::{Deserialize, Serialize};

use crate::printnanny::PrintNannySettings;

use super::error::PrintNannyCloudDataError;
use printnanny_api_client::models;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyCloudData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pi: Option<models::Pi>,
}

impl Default for PrintNannyCloudData {
    fn default() -> Self {
        PrintNannyCloudData { pi: None }
    }
}

impl PrintNannyCloudData {
    pub fn new() -> Result<PrintNannyCloudData, PrintNannyCloudDataError> {
        let settings = PrintNannySettings::new().unwrap();
        let result = Self::load(&settings.paths.cloud())?;
        Ok(result)
    }

    pub fn save(&self, file: &Path) -> Result<(), PrintNannyCloudDataError> {
        let data = serde_json::to_string_pretty(&self)?;
        match fs::write(file, data) {
            Ok(_) => Ok(()),
            Err(e) => Err(PrintNannyCloudDataError::WriteIOError {
                path: file.display().to_string(),
                error: e,
            }),
        }?;
        info!("Saved PrintNannyCloudData to {}", file.display());
        Ok(())
    }

    pub fn try_check_cloud_data(&self) -> Result<(), PrintNannyCloudDataError> {
        let settings = PrintNannySettings::new().unwrap();
        let state = PrintNannyCloudData::load(&settings.paths.cloud())?;
        match &state.pi {
            Some(_) => Ok(()),
            None => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: "pi".to_string(),
            }),
        }?;

        match &settings.cloud.api_bearer_access_token {
            Some(_) => Ok(()),
            None => Err(PrintNannyCloudDataError::SetupIncomplete {
                path: "cloud.api_bearer_access_token".to_string(),
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

    pub fn load(cloud: &Path) -> Result<PrintNannyCloudData, PrintNannyCloudDataError> {
        let state_str = match fs::read_to_string(cloud) {
            Ok(d) => Ok(d),
            Err(e) => Err(PrintNannyCloudDataError::ReadIOError {
                path: cloud.display().to_string(),
                error: e,
            }),
        }?;
        let state: PrintNannyCloudData = toml::de::from_str(&state_str)?;
        Ok(state)
    }
}

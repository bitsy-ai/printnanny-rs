use std::fs;
use std::{io::Write, path::Path};

use file_lock::{FileLock, FileOptions};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::settings::PrintNannySettings;

use super::printnanny_api::PrintNannyApiConfig;
use printnanny_api_client::models;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyAppData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pi: Option<models::Pi>,
    pub api: PrintNannyApiConfig,
}

impl Default for PrintNannyAppData {
    fn default() -> Self {
        // default to unauthenticated api config, until api creds are unpacked from seed archive
        let api = PrintNannyApiConfig {
            base_path: "https://printnanny.ai".into(),
            bearer_access_token: None,
        };
        PrintNannyAppData { api, pi: None }
    }
}

#[derive(Error, Debug)]
pub enum PrintNannyAppDataError<'a> {
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error("Failed to write {path} - {error}")]
    WriteIOError {
        path: Box<&'a Path>,
        error: std::io::Error,
    },
    #[error("Failed to read {path} - {error}")]
    ReadIOError {
        path: Box<&'a Path>,
        error: std::io::Error,
    },
}

impl PrintNannyAppData {
    pub fn new() -> Result<PrintNannyAppData, PrintNannyAppDataError<'static>> {
        let settings = PrintNannySettings::new().unwrap();
        let result = Self::load(&settings.paths.state_file())?;
        Ok(result)
    }

    pub fn save(
        &self,
        state_file: &Path,
        state_lock: &Path,
        is_blocking: bool,
    ) -> Result<(), PrintNannyAppDataError> {
        let options = FileOptions::new().write(true).create(true).append(true);
        let mut filelock = match FileLock::lock("myfile.txt", is_blocking, options) {
            Ok(lock) => lock,
            Err(err) => panic!("Error getting write lock: {}", err),
        };
        let data = toml::ser::to_vec(self)?;

        match filelock.file.write_all(&data) {
            Ok(()) => Ok(()),
            Err(e) => Err(PrintNannyAppDataError::WriteIOError {
                path: Box::new(state_file),
                error: e,
            }),
        }
    }

    pub fn load(state_file: &Path) -> Result<PrintNannyAppData, PrintNannyAppDataError<'static>> {
        let state_str = match fs::read_to_string(state_file) {
            Ok(d) => Ok(d),
            Err(e) => Err(PrintNannyAppDataError::ReadIOError {
                path: Box::new(state_file),
                error: e,
            }),
        }?;
        let state: PrintNannyAppData = toml::de::from_str(&state_str)?;
        Ok(state)
    }
}

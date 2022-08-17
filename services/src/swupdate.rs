use anyhow::Result;
use async_process::{Command, Output};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use tempfile::Builder;

use printnanny_api_client::models::pi_software_update_payload_request::PiSoftwareUpdatePayloadRequest;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Swupdate {
    swu_url: String,
    version: String,
}

impl Swupdate {
    pub fn new(swu_url: String, version: String) -> Self {
        Self { swu_url, version }
    }

    // download to temporary directory, which will be cleaned up when program exits
    pub async fn download_file(&self) -> Result<(PathBuf, File)> {
        let tmp_dir = Builder::new().prefix("printnanny-swupdate").tempdir()?;
        let response = reqwest::get(&self.swu_url).await?;
        let (filename, mut dest) = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("tmp.bin");

            info!("Swupdate file to download: '{}'", fname);
            let fname = tmp_dir.path().join(fname);
            info!("Swupdate file will be located under: '{:?}'", fname);
            let f = File::create(&fname)?;
            (fname, f)
        };
        let content = response.text().await?;
        std::io::copy(&mut content.as_bytes(), &mut dest)?;
        Ok((filename, dest))
    }

    pub async fn run(&self) -> Result<Output> {
        let (path, _f) = self.download_file().await?;

        let output = Command::new("swupdate")
            .args(&["-v", "-i", path.to_str().unwrap()])
            .output()
            .await?;
        Ok(output)
    }
}

impl From<PiSoftwareUpdatePayloadRequest> for Swupdate {
    fn from(payload: PiSoftwareUpdatePayloadRequest) -> Self {
        Self {
            swu_url: payload.swu_url,
            version: payload.version,
        }
    }
}

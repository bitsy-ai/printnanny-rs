use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use printnanny_settings::printnanny::PrintNannySettings;

use crate::error::ServiceError;
use crate::octoprint;

pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(&path).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to open file at {:?}: {}", path.as_ref(), err),
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VideoRecording {
    pub path: PathBuf,
    pub ts: u64,
}

pub async fn new_video_filename() -> Result<VideoRecording, ServiceError> {
    let settings = PrintNannySettings::new()?;

    // is octoprint api key set?
    // is octoprint running a job?
    let octoprint_current_job_filename = octoprint::octoprint_get_current_job_filename().await?;
    match octoprint_current_job_filename {
        Some(filename) => {
            let start = SystemTime::now();
            let ts = start
                .duration_since(UNIX_EPOCH)
                .expect("Failed to get UNIX_EPOCH")
                .as_secs();
            Ok(VideoRecording {
                path: settings.paths.video().join(filename),
                ts,
            })
        }
        None => {
            let start = SystemTime::now();
            let ts = start
                .duration_since(UNIX_EPOCH)
                .expect("Failed to get UNIX_EPOCH")
                .as_secs();
            Ok(VideoRecording {
                path: settings.paths.video().join("camera"), // TODO get camera label/display name
                ts,
            })
        }
    }

    // is moonraker / klipper running a job?

    // otherwise, return the selected camera's display name
}

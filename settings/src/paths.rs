use bytes::Bytes;
use figment::providers::Env;
use log::info;
use serde;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};
use zip::ZipArchive;

use super::error::PrintNannySettingsError;

pub const DEFAULT_PRINTNANNY_USER: &str = "printnanny";
pub const PRINTNANNY_SETTINGS_FILENAME: &str = "printnanny.toml";
pub const DEFAULT_PRINTNANNY_SETTINGS_DIR: &str = "/home/printnanny/.config/printnanny/vcs";
pub const DEFAULT_PRINTNANNY_SETTINGS_FILE: &str =
    "/home/printnanny/.config/printnanny/vcs/printnanny/printnanny.toml";
pub const DEFAULT_PRINTNANNY_DATA_DIR: &str = "/home/printnanny/.local/share/printnanny";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyPaths {
    pub state_dir: PathBuf,    // application state
    pub settings_dir: PathBuf, // local git repo used to commit/revert changes to user-supplied config
    pub log_dir: PathBuf,      // application log dir
    pub run_dir: PathBuf,      // application runtime dir

    pub issue_txt: PathBuf,  // path to /etc/issue
    pub os_release: PathBuf, // oath to /etc/os-release
}

impl Default for PrintNannyPaths {
    fn default() -> Self {
        let settings_dir: PathBuf = DEFAULT_PRINTNANNY_SETTINGS_DIR.into();
        // /var/run/ is a temporary runtime directory, cleared after each boot
        let run_dir: PathBuf = "/var/run/printnanny".into();
        // /home persistent state directory, mounted as a r/w overlay fs. Application state is stored here and is preserved between upgrades.
        let state_dir: PathBuf = DEFAULT_PRINTNANNY_DATA_DIR.into();

        let issue_txt: PathBuf = "/etc/issue".into();
        let log_dir: PathBuf = "/var/log/printnanny".into();
        let os_release = "/etc/os-release".into();
        Self {
            settings_dir,
            issue_txt,
            state_dir,
            log_dir,
            os_release,
            run_dir,
        }
    }
}

impl PrintNannyPaths {
    pub fn cloud(&self) -> PathBuf {
        self.data().join("PrintNannyCloudData.json")
    }

    // lock acquired when modifying persistent application data
    pub fn state_lock(&self) -> PathBuf {
        self.run_dir.join("state.lock")
    }

    // user-facing settings file
    pub fn settings_file(&self) -> PathBuf {
        PathBuf::from(Env::var_or(
            "PRINTNANNY_SETTINGS",
            DEFAULT_PRINTNANNY_SETTINGS_FILE,
        ))
    }

    // secrets, keys, credentials dir
    pub fn creds(&self) -> PathBuf {
        self.state_dir.join("creds")
    }

    // data directory
    pub fn data(&self) -> PathBuf {
        self.state_dir.join("data")
    }

    // event adaptor used to bridge any sender -> cloud NATS
    pub fn events_socket(&self) -> PathBuf {
        self.run_dir.join("events.socket")
    }
    // cloud nats jwt
    pub fn cloud_nats_creds(&self) -> PathBuf {
        self.creds().join("printnanny-cloud-nats.creds")
    }

    // recovery direcotry
    pub fn recovery(&self) -> PathBuf {
        self.state_dir.join("recovery")
    }

    // media (videos)
    pub fn video(&self) -> PathBuf {
        self.state_dir.join("video")
    }

    pub fn license_zip(&self) -> PathBuf {
        self.creds().join("license.zip")
    }

    fn try_init(&self, dir: &Path) -> Result<(), io::Error> {
        match dir.exists() {
            true => {
                info!("Skipping mkdir, already exists: {}", dir.display());
                Ok(())
            }
            false => {
                info!("Creating directory: {}", dir.display());
                std::fs::create_dir_all(dir)
            }
        }
    }

    pub fn try_init_all(&self) -> Result<(), io::Error> {
        let dirs = vec![self.creds(), self.data(), self.recovery(), self.video()];

        for dir in dirs {
            self.try_init(&dir)?;
        }
        Ok(())
    }

    pub fn try_load_nats_creds(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(self.cloud_nats_creds())
    }

    // unpack license to credentials dir (defaults to /etc/printnanny/creds)
    // returns a Vector of unzipped file PathBuf
    pub fn unpack_license(&self) -> Result<[(String, PathBuf); 1], PrintNannySettingsError> {
        let license_zip = self.license_zip();
        let file = match std::fs::File::open(&license_zip) {
            Ok(f) => Ok(f),
            Err(error) => Err(PrintNannySettingsError::ReadIOError {
                path: license_zip.clone(),
                error,
            }),
        }?;
        info!("Unpacking {:?}", file);
        let mut archive = ZipArchive::new(file)?;

        // filenames configured in creds_bundle here: https://github.com/bitsy-ai/printnanny-webapp/blob/d33b99ede33f02b0282c006d5549ae6f76866da5/print_nanny_webapp/devices/services.py#L233
        let results = [(
            "printnanny-cloud-nats.creds".to_string(),
            self.cloud_nats_creds(),
        )];

        for (filename, dest) in results.iter() {
            // if target file already fails and --force flag not passed
            if dest.exists() {
                self.backup_file(dest)?;
            }
            // read filename from archive
            let file = archive.by_name(filename);
            let mut file = match file {
                Ok(f) => Ok(f),
                Err(_) => Err(PrintNannySettingsError::ArchiveMissingFile {
                    filename: filename.to_string(),
                    archive: license_zip.clone(),
                }),
            }?;

            let mut contents = String::new();

            match file.read_to_string(&mut contents) {
                Ok(_) => Ok(()),
                Err(error) => Err(PrintNannySettingsError::ReadIOError {
                    path: PathBuf::from(filename),
                    error,
                }),
            }?;

            match std::fs::write(dest, contents) {
                Ok(_) => Ok(()),
                Err(error) => Err(PrintNannySettingsError::WriteIOError {
                    path: PathBuf::from(filename),
                    error,
                }),
            }?;
            info!("Wrote seed file {:?}", dest);
        }
        Ok(results)
    }

    // copy file contents to filename.ts.bak
    pub fn backup_file(&self, filename: &PathBuf) -> Result<PathBuf, PrintNannySettingsError> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_filename = format!("{}.{}.bak", filename.display(), ts);
        let new_filepath = PathBuf::from(&new_filename);
        fs::copy(filename, &new_filepath)?;
        info!(
            "{} already exists, backed up to {} before overwriting",
            filename.display(),
            new_filepath.display()
        );
        Ok(new_filepath)
    }

    pub fn write_license_zip(&self, b: Bytes) -> Result<(), PrintNannySettingsError> {
        let filename = self.license_zip();

        // if license.zip already exists, back up existing file before overwriting
        if filename.exists() {
            self.backup_file(&filename)?;
        }

        fs::write(filename, b)?;

        Ok(())
    }

    pub fn crash_report_paths(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/var/log/syslog"),
            PathBuf::from("/var/log/cloud-init.log"),
            PathBuf::from("/var/log/nginx/access.log"),
            PathBuf::from("/var/log/nginx/error.log"),
            PathBuf::from("/var/log/octoprint/"),
            PathBuf::from("/var/log/klipper/"),
            PathBuf::from("/var/log/moonraker/"),
            PathBuf::from("/etc/issue"),
            PathBuf::from("/etc/os-release"),
        ]
    }
}

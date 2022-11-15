extern crate glob;
use super::os_release::OsRelease;
use bytes::Bytes;
use log::info;
use serde;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

use super::error::PrintNannyConfigError;

pub const PRINTNANNY_CONFIG_FILENAME: &str = "default.toml";
pub const DEFAULT_PRINTNANNY_CONFIG: &str = "/etc/printnanny/default.toml";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct PrintNannyPaths {
    pub config_dir: PathBuf, // user-supplied config
    pub lib_dir: PathBuf,    // application state and default config
    pub log_dir: PathBuf,    // application log dir
    pub run_dir: PathBuf,    // application runtime dir

    pub issue_txt: PathBuf,  // path to /etc/issue
    pub os_release: PathBuf, // oath to /etc/os-release
}

impl Default for PrintNannyPaths {
    fn default() -> Self {
        // /etc is mounted as a read-only overlay fs. User configurations are stored here, and are preserved between upgrades.
        let config_dir: PathBuf = "/etc/printnanny.d".into();
        // /var/run/ is a temporary runtime directory, cleared after each boot
        let run_dir: PathBuf = "/var/run/printnanny".into();
        // /var/lib is a persistent state directory, mounted as a r/w overlay fs. Application state is stored here and is preserved between upgrades.
        let lib_dir: PathBuf = "/var/lib/printnanny".into();

        let issue_txt: PathBuf = "/etc/issue".into();
        let log_dir: PathBuf = "/var/log/printnanny".into();
        let os_release = "/etc/os-release".into();
        Self {
            config_dir,
            issue_txt,
            lib_dir,
            log_dir,
            os_release,
            run_dir,
        }
    }
}

impl PrintNannyPaths {
    // lock acquired when modifying persistent application data
    pub fn state_lock(&self) -> PathBuf {
        self.run_dir.join("state.lock")
    }

    // secrets, keys, credentials dir
    pub fn creds(&self) -> PathBuf {
        self.lib_dir.join("creds")
    }

    // data directory
    pub fn data(&self) -> PathBuf {
        self.lib_dir.join("data")
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
        self.lib_dir.join("recovery")
    }

    // media (videos)
    pub fn video(&self) -> PathBuf {
        self.data().join("video")
    }

    pub fn lib_confd(&self) -> PathBuf {
        self.lib_dir.join("printnanny.d")
    }

    pub fn user_confd(&self) -> PathBuf {
        self.config_dir.clone()
    }

    pub fn license_zip(&self) -> PathBuf {
        self.creds().join("license.zip")
    }

    pub fn try_load_nats_creds(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(self.cloud_nats_creds())
    }
    pub fn load_os_release(&self) -> Result<OsRelease, std::io::Error> {
        OsRelease::new_from(&self.os_release)
    }

    // unpack license to credentials dir (defaults to /etc/printnanny/creds)
    // returns a Vector of unzipped file PathBuf
    pub fn unpack_license(&self) -> Result<[(String, PathBuf); 1], PrintNannyConfigError> {
        let license_zip = self.license_zip();
        let file = match std::fs::File::open(&license_zip) {
            Ok(f) => Ok(f),
            Err(error) => Err(PrintNannyConfigError::ReadIOError {
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
                Err(_) => Err(PrintNannyConfigError::ArchiveMissingFile {
                    filename: filename.to_string(),
                    archive: license_zip.clone(),
                }),
            }?;

            let mut contents = String::new();

            match file.read_to_string(&mut contents) {
                Ok(_) => Ok(()),
                Err(error) => Err(PrintNannyConfigError::ReadIOError {
                    path: PathBuf::from(filename),
                    error,
                }),
            }?;

            match std::fs::write(&dest, contents) {
                Ok(_) => Ok(()),
                Err(error) => Err(PrintNannyConfigError::WriteIOError {
                    path: PathBuf::from(filename),
                    error,
                }),
            }?;
            info!("Wrote seed file {:?}", dest);
        }
        Ok(results)
    }

    // copy file contents to filename.ts.bak
    pub fn backup_file(&self, filename: &PathBuf) -> Result<PathBuf, PrintNannyConfigError> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_filename = format!("{}.{}.bak", filename.display(), ts);
        let new_filepath = PathBuf::from(&new_filename);
        fs::copy(&filename, &new_filepath)?;
        info!(
            "{} already exists, backed up to {} before overwriting",
            filename.display(),
            new_filepath.display()
        );
        Ok(new_filepath)
    }

    pub fn write_license_zip(&self, b: Bytes) -> Result<(), PrintNannyConfigError> {
        let filename = self.license_zip();

        // if license.zip already exists, back up existing file before overwriting
        if filename.exists() {
            self.backup_file(&filename)?;
        }

        fs::write(filename, b)?;

        Ok(())
    }
}

// serialize function path representation
impl serde::Serialize for PrintNannyPaths {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Extended {
            // extended fields
            pub config_dir: PathBuf,
            pub data: PathBuf,
            pub events_socket: PathBuf,
            pub issue_txt: PathBuf,
            pub lib_confd: PathBuf,
            pub lib_dir: PathBuf,
            pub log_dir: PathBuf,
            pub nats_creds: PathBuf,
            pub os_release: PathBuf,
            pub recovery: PathBuf,
            pub run_dir: PathBuf,
            pub state_lock: PathBuf,
            pub user_confd: PathBuf,
        }

        let ext = Extended {
            config_dir: self.config_dir.clone(),
            data: self.data(),
            events_socket: self.events_socket(),
            issue_txt: self.issue_txt.clone(),
            lib_confd: self.lib_confd(),
            lib_dir: self.lib_dir.clone(),
            log_dir: self.log_dir.clone(),
            nats_creds: self.cloud_nats_creds(),
            os_release: self.os_release.clone(),
            recovery: self.recovery(),
            run_dir: self.run_dir.clone(),
            state_lock: self.state_lock(),
            user_confd: self.user_confd(),
        };

        ext.serialize(serializer)
    }
}

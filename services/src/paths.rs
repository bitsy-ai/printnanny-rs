extern crate glob;
use self::glob::glob;
use super::os_release::OsRelease;
use log::info;
use serde;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use zip::ZipArchive;

use chrono::{DateTime, Utc}; // 0.4.15
use std::time::SystemTime;

use super::error::PrintNannyConfigError;

pub const PRINTNANNY_CONFIG_FILENAME: &str = "default.toml";
pub const PRINTNANNY_CONFIG_DEFAULT: &str = "/etc/printnanny/default.toml";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PrintNannyPaths {
    pub etc: PathBuf,
    pub seed_file_pattern: String,
    pub issue_txt: PathBuf,
    pub log: PathBuf,
    pub run: PathBuf,
    pub os_release: PathBuf,
}

impl Default for PrintNannyPaths {
    fn default() -> Self {
        // /etc is mounted as an r/w overlay fs
        let etc: PathBuf = "/etc/printnanny".into();
        let issue_txt: PathBuf = "/etc/issue".into();
        let run: PathBuf = "/var/run/printnanny".into();
        let log: PathBuf = "/var/log/printnanny".into();
        let seed_file_pattern = "/boot/printnanny*.zip".into();
        let os_release = "/etc/os-release".into();
        let video_socket = run.join("video.socket");
        Self {
            etc,
            run,
            issue_txt,
            log,
            seed_file_pattern,
            os_release,
        }
    }
}

impl PrintNannyPaths {
    pub fn new_video_filename(&self) -> PathBuf {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        let now = now.to_rfc3339();
        self.video().join(format!("{}.h264", now))
    }

    pub fn video_socket(&self) -> PathBuf {
        self.run.join("video.socket")
    }
    pub fn events_socket(&self) -> PathBuf {
        self.run.join("events.socket")
    }
    pub fn confd_lock(&self) -> PathBuf {
        self.run.join("confd.lock")
    }

    pub fn video(&self) -> PathBuf {
        self.data().join("video")
    }

    pub fn data(&self) -> PathBuf {
        self.etc.join("data")
    }

    pub fn recovery(&self) -> PathBuf {
        self.etc.join("recovery")
    }

    pub fn creds(&self) -> PathBuf {
        self.etc.join("creds")
    }

    pub fn confd(&self) -> PathBuf {
        self.etc.join("conf.d")
    }

    pub fn nats_creds(&self) -> PathBuf {
        self.creds().join("nats.creds")
    }

    pub fn license(&self) -> PathBuf {
        self.creds().join("license.json")
    }

    pub fn try_init_dirs(&self) -> Result<(), PrintNannyConfigError> {
        let dirs = [
            &self.etc,
            &self.recovery(),
            &self.data(),
            &self.creds(),
            &self.confd(),
            &self.video(),
            &self.run,
            &self.log,
        ];

        for dir in dirs.iter() {
            match dir.exists() {
                true => {
                    info!("Skipping mkdir, directory {:?} already exists", dir);
                    Ok(())
                }
                false => match fs::create_dir(&dir) {
                    Ok(()) => {
                        info!("Created directory {:?}", &dir);
                        Ok(())
                    }
                    Err(error) => Err(PrintNannyConfigError::WriteIOError {
                        path: dir.to_path_buf(),
                        error,
                    }),
                },
            }?;
        }
        Ok(())
    }
    pub fn try_load_nats_creds(&self) -> Result<String, std::io::Error> {
        std::fs::read_to_string(self.nats_creds())
    }
    pub fn load_os_release(&self) -> Result<OsRelease, std::io::Error> {
        OsRelease::new_from(&self.os_release)
    }

    fn try_find_seed(&self, pattern: &str) -> Result<PathBuf, PrintNannyConfigError> {
        // find seed file zip using glob pattern
        // the zip file is named PrintNanny-${hostname}.zip to make it easy for users to differentiate configs for multiple Pis
        let matched_zip = glob(pattern);
        let mut matched_zip = match matched_zip {
            Ok(v) => Ok(v),
            Err(_) => Err(PrintNannyConfigError::PatternNotFound {
                pattern: pattern.to_string(),
            }),
        }?;

        let matched_zip = matched_zip.next();
        match matched_zip {
            Some(result) => match result {
                Ok(v) => Ok(v),
                Err(_) => Err(PrintNannyConfigError::PatternNotFound {
                    pattern: pattern.to_string(),
                }),
            },
            None => Err(PrintNannyConfigError::PatternNotFound {
                pattern: pattern.to_string(),
            }),
        }
    }

    // backup PrintNanny.zip to data partition
    pub fn try_copy_seed(&self, force: bool) -> Result<(), PrintNannyConfigError> {
        let matched_zip = self.try_find_seed(&self.seed_file_pattern)?;
        let filename = matched_zip.file_name().unwrap();
        let dest = self.recovery().join(filename);
        if !(dest).exists() || force {
            match fs::copy(&matched_zip, &dest) {
                Ok(_) => {
                    info!("Copied {:?} to {:?}", &matched_zip, &dest);
                    Ok(())
                }
                Err(error) => Err(PrintNannyConfigError::CopyIOError {
                    src: matched_zip,
                    dest,
                    error,
                }),
            }
        } else {
            Err(PrintNannyConfigError::FileExists { path: dest })
        }
    }

    // unpack seed file to printnanny conf.d and credentials dir (defaults to /etc/printnanny/data)
    // returns a Vector of unzipped file PathBuf
    pub fn unpack_seed(
        &self,
        force: bool,
    ) -> Result<[(String, PathBuf); 2], PrintNannyConfigError> {
        let matched_zip = self.try_find_seed(&self.seed_file_pattern)?;
        let file = match std::fs::File::open(&matched_zip) {
            Ok(f) => Ok(f),
            Err(error) => Err(PrintNannyConfigError::ReadIOError {
                path: matched_zip.clone(),
                error,
            }),
        }?;
        info!("Unpacking seed zip {:?}", file);
        let mut archive = ZipArchive::new(file)?;

        // filenames configured in creds_bundle here: https://github.com/bitsy-ai/printnanny-webapp/blob/d33b99ede33f02b0282c006d5549ae6f76866da5/print_nanny_webapp/devices/services.py#L233

        let results = [
            ("license.json".to_string(), self.license()),
            ("nats.creds".to_string(), self.creds().join("nats.creds")),
        ];

        for (filename, dest) in results.iter() {
            // if target file already fails and --force flag not passed
            if dest.exists() && !force {
                return Err(PrintNannyConfigError::FileExists {
                    path: dest.to_path_buf(),
                });
            }
            // read filename from archive
            let file = archive.by_name(filename);
            let mut file = match file {
                Ok(f) => Ok(f),
                Err(_) => Err(PrintNannyConfigError::ArchiveMissingFile {
                    filename: filename.to_string(),
                    archive: matched_zip.clone(),
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
}

// serialize function path representation
impl serde::Serialize for PrintNannyPaths {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Extended {
            pub etc: PathBuf,
            pub seed_file_pattern: String,
            pub issue_txt: PathBuf,
            pub log: PathBuf,
            pub run: PathBuf,
            pub os_release: PathBuf,
            // extended fields
            pub video_socket: PathBuf,
            pub events_socket: PathBuf,
            pub confd_lock: PathBuf,
            pub data: PathBuf,
            pub recovery: PathBuf,
            pub nats_creds: PathBuf,
            pub license: PathBuf,
        }

        let ext = Extended {
            video_socket: self.video_socket(),
            events_socket: self.events_socket(),
            confd_lock: self.confd_lock(),
            data: self.data(),
            recovery: self.recovery(),
            nats_creds: self.nats_creds(),
            license: self.license(),

            etc: self.etc.clone(),
            seed_file_pattern: self.seed_file_pattern.clone(),
            issue_txt: self.issue_txt.clone(),
            log: self.log.clone(),
            run: self.run.clone(),
            os_release: self.os_release.clone(),
        };

        Ok(ext.serialize(serializer)?)
    }
}

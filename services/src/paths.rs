use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const OCTOPRINT_DIR: &str = "/home/octoprint/.octoprint";
pub const PRINTNANNY_CONFIG_FILENAME: &str = "default.toml";
pub const PRINTNANNY_CONFIG_DEFAULT: &str = "/etc/printnanny/default.toml";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyPaths {
    pub etc: PathBuf,
    pub confd: PathBuf,
    pub events_socket: PathBuf,
    pub license: PathBuf,
    pub issue_txt: PathBuf,
    pub log: PathBuf,
    pub octoprint: PathBuf,
    pub run: PathBuf,
    pub os_release: PathBuf,
}

impl Default for PrintNannyPaths {
    fn default() -> Self {
        // /etc is mounted as an r/w overlay fs
        let etc: PathBuf = "/etc/printnanny".into();
        let confd: PathBuf = "/etc/printnanny/conf.d".into();
        let issue_txt: PathBuf = "/boot/issue.txt".into();
        let run: PathBuf = "/var/run/printnanny".into();
        let log: PathBuf = "/var/log/printnanny".into();
        let events_socket = run.join("events.socket").into();
        let license = "/boot/license.json".into();
        let octoprint = OCTOPRINT_DIR.into();
        let os_release = "/etc/os-release".into();
        Self {
            etc,
            confd,
            run,
            issue_txt,
            log,
            events_socket,
            octoprint,
            license,
            os_release,
        }
    }
}

impl PrintNannyPaths {
    pub fn data(&self) -> PathBuf {
        self.etc.join("data")
    }
    pub fn octoprint_venv(&self) -> PathBuf {
        self.octoprint.join("venv")
    }

    pub fn octoprint_pip(&self) -> PathBuf {
        self.octoprint_venv().join("bin/pip")
    }

    pub fn octoprint_python(&self) -> PathBuf {
        self.octoprint_venv().join("bin/pip")
    }
}

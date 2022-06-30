use super::os_release::OsRelease;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PRINTNANNY_CONFIG_FILENAME: &str = "default.toml";
pub const PRINTNANNY_CONFIG_DEFAULT: &str = "/etc/printnanny/default.toml";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyPaths {
    pub etc: PathBuf,
    pub confd: PathBuf,
    pub confd_lock: PathBuf,
    pub events_socket: PathBuf,
    pub seed: PathBuf,
    pub issue_txt: PathBuf,
    pub log: PathBuf,
    pub run: PathBuf,
    pub os_release: PathBuf,
}

impl Default for PrintNannyPaths {
    fn default() -> Self {
        // /etc is mounted as an r/w overlay fs
        let etc: PathBuf = "/etc/printnanny".into();
        let confd: PathBuf = "/etc/printnanny/conf.d".into();
        let issue_txt: PathBuf = "/etc/issue".into();
        let run: PathBuf = "/var/run/printnanny".into();
        let log: PathBuf = "/var/log/printnanny".into();
        let events_socket = run.join("events.socket").into();
        let seed = "/boot/PrintNanny.toml".into();
        let os_release = "/etc/os-release".into();
        let confd_lock = run.join("confd.lock");
        Self {
            etc,
            confd,
            run,
            issue_txt,
            log,
            events_socket,
            seed,
            os_release,
            confd_lock,
        }
    }
}

impl PrintNannyPaths {
    pub fn data(&self) -> PathBuf {
        self.etc.join("data")
    }
    pub fn load_os_release(&self) -> Result<OsRelease, std::io::Error> {
        OsRelease::new_from(&self.os_release)
    }
}

use super::os_release::OsRelease;
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

    // Parse /etc/os-release into OsRelease
    // Strips out quotes
    // pub fn os_release(&self) -> Result<OsRelease, std::io::Error> {
    //     let content = fs::read_to_string(&self.os_release)?;
    //     let mut map = HashMap::<String, Value>::new();
    //     let lines = content.split("\n");
    //     for line in (lines).step_by(1) {
    //         if line.contains("=") {
    //             let mut pair = line.split("=");
    //             let key = pair.nth(0).unwrap_or("unknown").to_string();
    //             let value = pair
    //                 .nth(0)
    //                 .unwrap_or("unknown")
    //                 .replace("\"", "")
    //                 .to_string();
    //             map.insert(key, Value::from(value));
    //         }
    //     }
    //     info!("Parsed Map from {:?}: {:?}", &self.os_release, map);
    //     let result: OsRelease  =
    //     Ok(OsRelease::from(map))
    // }

    pub fn load_os_release(&self) -> Result<OsRelease, std::io::Error> {
        OsRelease::new_from(&self.os_release)
    }
}

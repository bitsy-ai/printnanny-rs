use std::path::{Path, PathBuf};
use tokio::process::Command;

use async_trait::async_trait;
use figment::providers::Env;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use crate::error::PrintNannySettingsError;
use crate::error::VersionControlledSettingsError;
use crate::printnanny::GitSettings;
use crate::vcs::VersionControlledSettings;
use crate::SettingsFormat;

pub const OCTOPRINT_INSTALL_DIR: &str = "/home/printnanny/.octoprint";
pub const OCTOPRINT_VENV: &str = "/home/printnanny/octoprint-venv";
pub const DEFAULT_OCTOPRINT_SETTINGS_FILE: &str =
    "/home/printnanny/.config/printnanny/settings/octoprint/octoprint.yaml";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipPackage {
    name: String,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OctoPrintSettings {
    pub enabled: bool,
    pub install_dir: PathBuf,
    pub settings_file: PathBuf,
    pub settings_format: SettingsFormat,
    pub venv: PathBuf,
    pub git_settings: GitSettings,
}

impl OctoPrintSettings {
    pub fn new(
        enabled: bool,
        install_dir: PathBuf,
        settings_file: PathBuf,
        settings_format: SettingsFormat,
        venv: PathBuf,
        git_settings: GitSettings,
    ) -> Self {
        Self {
            enabled,
            install_dir,
            settings_file,
            settings_format,
            venv,
            git_settings,
        }
    }
}

#[async_trait]
impl VersionControlledSettings for OctoPrintSettings {
    type SettingsModel = OctoPrintSettings;
    fn from_dir(settings_dir: &Path) -> Self {
        let settings_file = settings_dir.join("octoprint/octoprint.yaml");
        Self {
            settings_file,
            ..Self::default()
        }
    }
    fn get_settings_format(&self) -> SettingsFormat {
        self.settings_format
    }
    fn get_settings_file(&self) -> PathBuf {
        self.settings_file.clone()
    }

    fn get_git_repo_path(&self) -> &Path {
        &self.git_settings.path
    }

    fn get_git_remote(&self) -> &str {
        &self.git_settings.remote
    }

    fn get_git_settings(&self) -> &GitSettings {
        &self.git_settings
    }

    async fn pre_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running OctoPrintSettings pre_save hook");
        // stop OctoPrint serviice
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .stop_unit("octoprint.service".to_string(), "replace".to_string())
            .await?;
        info!("Stopped octoprint.service, job: {:?}", job);
        Ok(())
    }

    async fn post_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running KlipperSettings post_save hook");
        // start OctoPrint service
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .start_unit("octoprint.service".into(), "replace".into())
            .await?;
        info!("Started octoprint.service, job: {:?}", job);

        Ok(())
    }
    fn validate(&self) -> Result<(), VersionControlledSettingsError> {
        todo!("OctoPrintSettings validate hook is not yet implemented");
    }
}

impl Default for OctoPrintSettings {
    fn default() -> Self {
        let install_dir: PathBuf = OCTOPRINT_INSTALL_DIR.into();
        let settings_file = PathBuf::from(Env::var_or(
            "OCTOPRINT_SETTINGS_FILE",
            DEFAULT_OCTOPRINT_SETTINGS_FILE,
        ));
        let git_settings = GitSettings::default();
        Self {
            settings_file,
            install_dir,
            enabled: true,
            venv: OCTOPRINT_VENV.into(),
            settings_format: SettingsFormat::Yaml,
            git_settings,
        }
    }
}

pub fn parse_pip_list_json(stdout: &str) -> Result<Vec<PipPackage>, PrintNannySettingsError> {
    let v: Vec<PipPackage> = serde_json::from_str(stdout)?;
    Ok(v)
}

// parse output of:
// $ python3 --version
// Python 3.10.4
pub fn parse_python_version(stdout: &str) -> Option<String> {
    stdout
        .split_once(' ')
        .map(|(_, version)| version.to_string())
}
pub fn parse_pip_version(stdout: &str) -> Option<String> {
    let split = stdout.split(' ').nth(1);
    split.map(|v| v.to_string())
}

impl OctoPrintSettings {
    pub fn python_path(&self) -> PathBuf {
        self.venv.join("bin/python")
    }

    pub async fn pip_version(&self) -> Result<Option<String>, PrintNannySettingsError> {
        let python_path = self.python_path();
        let output = Command::new(&python_path)
            .arg("-m")
            .arg("pip")
            .arg("--version")
            .output()
            .await;
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_pip_version(&stdout);
                debug!(
                    "Found pip_packages in venv {:?} {:?}",
                    &python_path, &result
                );
                Ok(result)
            }
            Err(e) => {
                let msg = format!(
                    "{:?} -m pip --version failed with error={}",
                    &python_path, e
                );
                error!("{}", &msg);
                Ok(None)
            }
        }
    }

    pub async fn pip_packages(&self) -> Result<Vec<PipPackage>, PrintNannySettingsError> {
        let python_path = self.python_path();
        let output = Command::new(&python_path)
            .arg("-m")
            .arg("pip")
            .arg("list")
            .arg("--include-editable") // handle dev environment, where pip install -e . is used for plugin setup
            .arg("--format")
            .arg("json")
            .output()
            .await;
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_pip_list_json(&stdout)?;
                debug!(
                    "Found pip_packages for python at {:?} {:?}",
                    &python_path, &result
                );
                Ok(result)
            }
            Err(e) => {
                let msg = format!(
                    "{} -m pip list --include-editable --format json failed with error={}",
                    &python_path.display(),
                    e
                );
                error!("{}", &msg);
                Ok(vec![])
            }
        }
    }

    pub async fn python_version(&self) -> Result<Option<String>, PrintNannySettingsError> {
        let python_path = self.python_path();
        let output = Command::new(&python_path).arg("--version").output().await;
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_python_version(&stdout);
                debug!("Parsed python_version in {:?} {:?}", &python_path, &result);
                Ok(result)
            }
            Err(e) => {
                debug!(
                    "Failed to parse {} --version with error={}",
                    &python_path.display(),
                    &e
                );
                Ok(None)
            }
        }
    }

    pub fn octoprint_version(&self, packages: &[PipPackage]) -> Option<String> {
        let python_path = self.python_path();

        let v: Vec<&PipPackage> = packages.iter().filter(|p| p.name == "OctoPrint").collect();
        match v.first() {
            Some(p) => {
                debug!(
                    "Parsed octoprint_version {:?} in venv {:?} ",
                    &p, &python_path
                );
                Some(p.version.clone())
            }
            None => {
                error!("Failed to parse octoprint version from pip output");
                None
            }
        }
    }

    pub fn printnanny_plugin_version(&self, packages: &[PipPackage]) -> Option<String> {
        let python_path = self.python_path();

        let v: Vec<&PipPackage> = packages
            .iter()
            .filter(|p| p.name == "OctoPrint-Nanny")
            .collect();
        match v.first() {
            Some(p) => {
                debug!(
                    "Parsed printnnny_plugin_version {:?} in venv {:?} ",
                    &p, python_path
                );
                Some(p.version.clone())
            }
            None => {
                error!("Failed to parse octoprint-nanny plugin version with pip");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"[{"name": "apturl", "version": "0.5.2"}, {"name": "astroid", "version": "2.9.3"}]
"#;

    #[test]
    fn test_pip_packages() {
        let actual = parse_pip_list_json(EXAMPLE.into()).unwrap();
        let expected = vec![
            PipPackage {
                name: "apturl".into(),
                version: "0.5.2".into(),
            },
            PipPackage {
                name: "astroid".into(),
                version: "2.9.3".into(),
            },
        ];

        assert_eq!(actual, expected)
    }
}

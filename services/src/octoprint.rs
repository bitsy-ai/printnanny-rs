use log::debug;
use std::path::PathBuf;
use std::process::Command;

use printnanny_api_client::models;
use serde::{Deserialize, Serialize};

use super::error::PrintNannyConfigError;

pub const OCTOPRINT_BASE_PATH: &str = "/home/octoprint/.octoprint";
pub const PYTHON_BIN: &str = "/usr/bin/python3";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipPackage {
    name: String,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintConfig {
    pub server: Option<models::OctoPrintServer>,
    pub base_path: PathBuf,
    pub python: PathBuf,
}

impl Default for OctoPrintConfig {
    fn default() -> Self {
        Self {
            server: None,
            base_path: OCTOPRINT_BASE_PATH.into(),
            python: PYTHON_BIN.into(),
        }
    }
}

pub fn parse_pip_list_json(stdout: &str) -> Result<Vec<PipPackage>, PrintNannyConfigError> {
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

// parse output of:
// $ pip --version
// pip 22.0.2 from /usr/lib/python3/dist-packages/pip (python 3.10)
pub fn parse_pip_version(stdout: &str) -> Option<String> {
    let split = stdout.split(' ').nth(1);
    split.map(|v| v.to_string())
}

impl OctoPrintConfig {
    // return boolean indicating whether PrintNanny OS edition requires OctoPrintConfig
    pub fn required(variant_id: &str) -> bool {
        match variant_id {
            "octoprint" => true,
            _ => false,
        }
    }
    pub fn pip_version(&self) -> Result<Option<String>, PrintNannyConfigError> {
        let msg = format!("{:?} -m pip --version failed", &self.python);
        let output = Command::new(&self.python)
            .arg("-m")
            .arg("pip")
            .arg("--version")
            .output()
            .expect(&msg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        match output.status.success() {
            true => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_pip_version(&stdout);
                debug!(
                    "Found pip_packages in venv {:?} {:?}",
                    &self.python, &result
                );
                Ok(result)
            }
            false => {
                let code = output.status.code();
                let stderr = String::from_utf8_lossy(&output.stderr).into();
                let stdout = stdout.into();
                Err(PrintNannyConfigError::CommandError {
                    cmd: msg,
                    stdout,
                    stderr,
                    code,
                })
            }
        }
    }

    pub fn pip_packages(&self) -> Result<Vec<PipPackage>, PrintNannyConfigError> {
        let output = Command::new(&self.python)
            .arg("-m")
            .arg("pip")
            .arg("list")
            .arg("--include-editable") // handle dev environment, where pip install -e . is used for plugin setup
            .arg("--format")
            .arg("json")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        match output.status.success() {
            true => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_pip_list_json(&stdout)?;
                debug!(
                    "Found pip_packages for python at {:?} {:?}",
                    &self.python, &result
                );
                Ok(result)
            }
            false => {
                let cmd = format!("{:?} -m pip list --format json", &self.python);
                let code = output.status.code();
                let stderr = String::from_utf8_lossy(&output.stderr).into();
                let stdout = stdout.into();
                Err(PrintNannyConfigError::CommandError {
                    cmd,
                    stdout,
                    stderr,
                    code,
                })
            }
        }
    }

    pub fn python_version(&self) -> Result<Option<String>, PrintNannyConfigError> {
        let msg = format!("{:?} --version failed", &self.python);
        let output = Command::new(&self.python)
            .arg("--version")
            .output()
            .expect(&msg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        match output.status.success() {
            true => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = parse_python_version(&stdout);
                debug!("Parsed python_version in {:?} {:?}", &self.python, &result);
                Ok(result)
            }
            false => {
                let cmd = format!("{:?} --version", &self.python);
                let code = output.status.code();
                let stderr = String::from_utf8_lossy(&output.stderr).into();
                let stdout = stdout.into();
                Err(PrintNannyConfigError::CommandError {
                    cmd,
                    stdout,
                    stderr,
                    code,
                })
            }
        }
    }

    pub fn octoprint_version(
        &self,
        packages: &[PipPackage],
    ) -> Result<String, PrintNannyConfigError> {
        let v: Vec<&PipPackage> = packages.iter().filter(|p| p.name == "OctoPrint").collect();
        let result = match v.first() {
            Some(p) => Ok(p.version.clone()),
            None => Err(PrintNannyConfigError::OctoPrintServerConfigError {
                field: "octoprint_version".into(),
                detail: None,
            }),
        }?;
        debug!(
            "Parsed octoprint_version {:?} in venv {:?} ",
            &result, &self.python
        );
        Ok(result)
    }

    pub fn printnanny_plugin_version(
        &self,
        packages: &[PipPackage],
    ) -> Result<Option<String>, PrintNannyConfigError> {
        let v: Vec<&PipPackage> = packages
            .iter()
            .filter(|p| p.name == "OctoPrint-Nanny")
            .collect();
        let result = match v.first() {
            Some(p) => Ok(p.version.clone()),
            None => Err(PrintNannyConfigError::OctoPrintServerConfigError {
                field: "printnanny_plugin_version".into(),
                detail: None,
            }),
        }?;
        debug!(
            "Parsed printnnny_plugin_version {:?} in venv {:?} ",
            &result, &self.python
        );
        Ok(Some(result))
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

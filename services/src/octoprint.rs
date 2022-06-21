use log::debug;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use printnanny_api_client::models;
use serde::{Deserialize, Serialize};

use super::error::PrintNannyConfigError;

pub const OCTOPRINT_BASE_PATH: &str = "/home/octoprint/.octoprint";
pub const OCTOPRINT_VENV_DIR: &str = "/home/octoprint/.venv";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipPackage {
    name: String,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintConfig {
    pub server: Option<models::OctoPrintServer>,
    pub base_path: PathBuf,
    pub venv_path: PathBuf,
}

impl Default for OctoPrintConfig {
    fn default() -> Self {
        Self {
            server: None,
            base_path: OCTOPRINT_BASE_PATH.into(),
            venv_path: OCTOPRINT_VENV_DIR.into(),
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
    match stdout.split_once(" ") {
        Some((_, version)) => Some(version.to_string()),
        None => None,
    }
}

// parse output of:
// $ pip --version
// pip 22.0.2 from /usr/lib/python3/dist-packages/pip (python 3.10)
pub fn parse_pip_version(stdout: &str) -> Option<String> {
    let split = stdout.splitn(3, " ").nth(1);
    match split {
        Some(v) => Some(v.to_string()),
        None => None,
    }
}

impl OctoPrintConfig {
    pub fn pip_path(&self) -> PathBuf {
        self.venv_path.join("bin/pip")
    }

    pub fn python_path(&self) -> PathBuf {
        self.venv_path.join("bin/python")
    }

    pub fn pip_version(&self) -> Result<Option<String>, PrintNannyConfigError> {
        let msg = format!("{:?} --version failed", &self.pip_path());
        let output = Command::new(&self.pip_path())
            .arg("--version")
            .output()
            .expect(&msg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_pip_version(&stdout))
    }

    pub fn pip_packages(&self) -> Result<Vec<PipPackage>, PrintNannyConfigError> {
        let msg = format!("{:?} list --json failed", &self.pip_path());
        let output = Command::new(&self.pip_path())
            .arg("list")
            .arg("--json")
            .output()
            .expect(&msg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = parse_pip_list_json(&stdout)?;
        debug!(
            "Found pip_packages in venv {:?} {:?}",
            &self.venv_path, &result
        );
        Ok(result)
    }

    pub fn python_version(&self) -> Result<Option<String>, PrintNannyConfigError> {
        let msg = format!("{:?} --version failed", self.pip_path());
        let output = Command::new(&self.python_path())
            .arg("--version")
            .output()
            .expect(&msg);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = parse_python_version(&stdout);
        debug!(
            "Parsed python_version in {:?} {:?}",
            &self.venv_path, &result
        );
        Ok(result)
    }

    pub fn octoprint_version(
        &self,
        packages: &Vec<PipPackage>,
    ) -> Result<String, PrintNannyConfigError> {
        let v: Vec<&PipPackage> = packages
            .into_iter()
            .filter(|p| p.name == "OctoPrint")
            .collect();
        let result = match v.first() {
            Some(p) => Ok(p.version.clone()),
            None => Err(PrintNannyConfigError::OctoPrintServerConfigError {
                field: "octoprint_version".into(),
                detail: None,
            }),
        }?;
        debug!(
            "Parsed octoprint_version {:?} in venv {:?} ",
            &result, &self.venv_path
        );
        Ok(result)
    }
    pub fn printnanny_plugin_version(
        &self,
        packages: &Vec<PipPackage>,
    ) -> Result<String, PrintNannyConfigError> {
        let v: Vec<&PipPackage> = packages
            .into_iter()
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
            &result, &self.venv_path
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

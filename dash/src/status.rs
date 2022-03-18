use std::io;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CMD_SERVICE: &str = "printnanny-cmd";
pub const DASH_SERVICE: &str = "printnanny-dash";
pub const FIRSTBOOT_SERVICE: &str = "printnanny-firstboot";
pub const METADATA_SERVICE: &str = "printnanny-metadata";
pub const MONITOR_SERVICE: &str = "printnanny-monitor";
pub const MQTT_SERVICE: &str = "printnanny-mqtt";
pub const NGINX_SERVICE: &str = "printnanny-ngnx";
pub const OCTOPRINT_SERBICE: &str = "printnanny-octoprint";

pub const SERVICES: &'static [&str] = &[
    CMD_SERVICE,
    DASH_SERVICE,
    FIRSTBOOT_SERVICE,
    METADATA_SERVICE,
    MONITOR_SERVICE,
    MQTT_SERVICE,
];

#[derive(Error, Debug)]
pub enum HealthCheckError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitState {
    unit: String,
    load: String,
    active: String,
    sub: String,
    description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCheck {
    pub firstboot_ok: bool,
    pub list_units: Vec<UnitState>,
    pub boot_history: Vec<String>,
    pub systemctl_status: String,
}

impl HealthCheck {
    pub fn new() -> Result<Self, HealthCheckError> {
        let systemctl_status = Self::systemctl_status()?;
        let boot_history = Self::boot_history()?;
        let list_units = Self::list_units()?;
        let firstboot_ok = Self::firstboot_ok()?;
        let health_check = Self {
            boot_history,
            firstboot_ok,
            list_units,
            systemctl_status,
        };
        info!("HealthCheck.firstboot_ok {:?}", health_check.firstboot_ok);
        Ok(health_check)
    }

    pub fn firstboot_ok() -> Result<bool, HealthCheckError> {
        let args = &["show", "-p", "SubState", "--value", FIRSTBOOT_SERVICE];
        let output = Command::new("systemctl").args(args).output()?;
        let substate = String::from_utf8_lossy(&output.stdout);
        let args = &["show", "-p", "ExecMainStatus", "--value", FIRSTBOOT_SERVICE];
        let output = Command::new("systemctl").args(args).output()?;
        let status = String::from_utf8_lossy(&output.stdout);
        info!("firstbook_ok() substate={} status={}", substate, status);
        Ok(substate == "exited" && status == "0")
    }

    pub fn list_units() -> Result<Vec<UnitState>, HealthCheckError> {
        let args = &[
            "list-units",
            "printnanny*",
            "--no-pager",
            "--all", // list units in both active / inactive states
            "-o",
            "json",
        ];
        let output = Command::new("systemctl").args(args).output()?;
        info!("systemctl {:?} output {:?}", args, output);
        let result = serde_json::from_slice(output.stdout.as_slice());
        match result {
            Ok(r) => Ok(r),
            Err(e) => {
                warn!("Error running systemctl list-units printnanny* {:?}", e);
                Ok(vec![])
            }
        }
    }

    pub fn systemctl_status() -> Result<String, HealthCheckError> {
        let args = &[
            "status",
            "printnanny*",
            "--no-pager",
            "-l", // show full untruncated output
            "-o",
            "short-iso", // show dates in iso format
        ];
        let output = Command::new("systemctl").args(args).output()?;
        info!("systemctl {:?} output {:?}", args, output);

        let result = String::from_utf8(output.stdout)?;
        Ok(result)
    }

    pub fn boot_history() -> Result<Vec<String>, HealthCheckError> {
        let output = Command::new("journalctl")
            .args(&["--list-boots"])
            .output()?;
        let result = String::from_utf8_lossy(output.stdout.as_slice())
            .split('\n')
            .map(String::from)
            .collect::<Vec<String>>();
        Ok(result)
    }
}

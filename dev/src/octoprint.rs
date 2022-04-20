use anyhow::Result;
use clap::ArgEnum;
use printnanny_services::config::PrintNannyConfig;
use std::io::{self, Write};
use std::process::{Child, Command};

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum OctoPrintAction {
    PipInstall,
    PipUninstall,
}

impl OctoPrintAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        OctoPrintAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for OctoPrintAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

pub struct OctoPrintCmd {
    pub action: OctoPrintAction,
    pub config: PrintNannyConfig,
    pub package: Option<String>,
}

impl OctoPrintCmd {
    pub fn new(action: OctoPrintAction, config: PrintNannyConfig, package: Option<String>) -> Self {
        Self {
            action,
            config,
            package,
        }
    }

    pub fn handle_pip_install(self) -> Result<Child> {
        let package = &self.package.expect("package is required");
        let args = &["install", "--upgrade", "--force-reinstall", package];
        let cmd = self
            .config
            .paths
            .octoprint_pip()
            .expect("Failed to find octoprint pip");
        let output = Command::new(cmd).args(args).spawn()?;
        Ok(output)
    }
    pub fn handle_pip_uninstall(self) -> Result<Child> {
        let package = &self.package.expect("package is required");
        let args = &["uninstall", package];
        let cmd = self
            .config
            .paths
            .octoprint_pip()
            .expect("Failed to find octoprint pip");
        let output = Command::new(cmd).args(args).spawn()?;
        Ok(output)
    }
    pub fn handle(self) -> Result<Child> {
        match self.action {
            OctoPrintAction::PipInstall => self.handle_pip_install(),
            OctoPrintAction::PipUninstall => self.handle_pip_uninstall(),
        }
    }
}

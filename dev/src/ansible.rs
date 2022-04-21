use anyhow::Result;
use clap::ArgEnum;
use printnanny_services::config::PrintNannyConfig;
use std::process::{Child, Command};

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum AnsibleAction {
    SetProfile,
}

impl AnsibleAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        AnsibleAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for AnsibleAction {
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

pub struct AnsibleCmd {
    pub action: AnsibleAction,
    pub config: PrintNannyConfig,
    pub profile: Option<String>,
}

impl AnsibleCmd {
    pub fn new(action: AnsibleAction, config: PrintNannyConfig, profile: Option<String>) -> Self {
        Self {
            action,
            config,
            profile,
        }
    }

    fn stop_printnanny_setup(self) -> Result<Child> {
        let args = &["stop", "printnanny-setup.target"];
        Ok(Command::new("systemctl").args(args).spawn()?)
    }

    fn start_printnanny_setup(self) -> Result<Child> {
        let args = &["start", "printnanny-setup.target"];
        Ok(Command::new("systemctl").args(args).spawn()?)
    }

    fn stop_printnanny_services(self) -> Result<Child> {
        let args = &["stop", "printnanny.target"];
        Ok(Command::new("systemctl").args(args).spawn()?)
    }
    fn start_printnanny_services(self) -> Result<Child> {
        let args = &["start", "printnanny.target"];
        Ok(Command::new("systemctl").args(args).spawn()?)
    }
    fn set_profile_fact(self, profile: &str) -> Result<Child> {
        let args = &[
            "printnanny",
            "-m",
            "ansible.builtin.set_fact",
            "-a",
            format!("'printnanny_profile={} cacheable=true'", profile),
        ];
        Ok(Command::new(self.config.ansible.ansible())
            .args(args)
            .spawn()?)
    }

    pub fn handle_set_profile(self) -> Result<()> {
        let profile = &self.profile.expect("profile is required");
        self.stop_printnanny_setup()?;
        self.stop_printnanny_services()?;
        self.set_profile_fact(profile)?;
        self.start_printnanny_setup()?;
        self.start_printnanny_services()?;
        Ok(())
    }
    pub fn handle(self) -> Result<()> {
        match self.action {
            AnsibleAction::SetProfile => self.handle_set_profile(),
        }
    }
}

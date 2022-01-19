use std::fs;
use std::io;
use std::path::Path;

use anyhow::Result;
use clap::ArgEnum;
use log::debug;

use printnanny_api_client::models;
use printnanny_services::config::PrintNannyConfig;
use printnanny_services::printnanny_api::ApiService;

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum ProfileAction {
    List,
    Create,
    Activate,
}

impl ProfileAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        ProfileAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for ProfileAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid ProfileAction variant: {}", s))
    }
}

pub struct ProfileCmd {
    pub action: ProfileAction,
    pub config: PrintNannyConfig,
}

impl ProfileCmd {
    pub fn new(action: ProfileAction, path: String) -> Result<Self> {
        let config = PrintNannyConfig {
            path,
            ..PrintNannyConfig::default()
        };
        Ok(Self { action, config })
    }
    pub fn handle(&self) -> Result<()> {
        // let figment
        // let result = match self.action {
        //     ProfileAction::List => {

        //     },
        //     _ => "".to_string()
        // };
        Ok(())
    }
}

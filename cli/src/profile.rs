use std::fs;
use std::io;
use std::path::Path;

use anyhow::{ Result };
use log:: { debug };
use clap::ArgEnum;

use printnanny_services::config::Config;
use printnanny_services::printnanny_api::{ ApiConfig, ApiService};
use printnanny_api_client::models;

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum ProfileAction {
    List,
    Create,
    Activate
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
    pub config: Config
}

impl ProfileCmd {
    pub fn new(action: ProfileAction, prefix: String) -> Result<Self> {

        let config = Config{ prefix, ..Config.default()};
        Ok(Self{action, config})
    }
    pub fn handle(&self) -> Result<()> {
        let (profiles, active_profile) = match self.action {
            ProfileAction::List => self.config.list_profiles(),
            _ => Ok(())
        };
        
        Ok(())
    }

}
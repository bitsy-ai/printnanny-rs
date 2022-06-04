use anyhow::Result;
use clap::ArgEnum;
use log::debug;

use printnanny_api_client::models;
use printnanny_services::config::PrintNannyConfig;
use printnanny_services::printnanny_api::ApiService;

#[derive(Copy, Eq, PartialEq, Debug, Clone, clap::ArgEnum)]
pub enum DeviceAction {
    Get,
    Setup,
}

impl DeviceAction {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        DeviceAction::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
}

impl std::str::FromStr for DeviceAction {
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

pub struct DeviceCmd {}
impl DeviceCmd {
    pub async fn handle(action: DeviceAction) -> Result<String> {
        let config = PrintNannyConfig::new()?;
        let mut service = ApiService::new(config)?;
        let result = match action {
            DeviceAction::Get => service.device_retrieve_hostname().await?,
            DeviceAction::Setup => {
                service.device_setup().await?;
                service
                    .config
                    .device
                    .clone()
                    .expect("Failed to setup device")
            }
        };
        debug!("Success action={:?} result={:?}", action, result);
        Ok(serde_json::to_string_pretty::<models::Device>(&result)?)
    }
}

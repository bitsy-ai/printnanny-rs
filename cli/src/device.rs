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

pub struct DeviceCmd {
    pub action: DeviceAction,
    pub service: ApiService,
}
impl DeviceCmd {
    pub async fn new(action: DeviceAction, config: PrintNannyConfig) -> Result<Self> {
        let service = ApiService::new(config)?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String> {
        let result = match self.action {
            DeviceAction::Get => self.service.device_retrieve_hostname().await?,
            DeviceAction::Setup => self
                .service
                .device_setup()
                .await?
                .device
                .expect("Failed to setup device"),
        };
        debug!("Success action={:?} result={:?}", self.action, result);
        Ok(serde_json::to_string_pretty::<models::Device>(&result)?)
    }
}

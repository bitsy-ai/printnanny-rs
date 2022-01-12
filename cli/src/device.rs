use anyhow::{ Result };
use clap::arg_enum;
use log:: { debug };

use printnanny_services::printnanny_api::ApiService;
use printnanny_api_client::models;

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum DeviceAction{
        Get,
    }
}

pub struct DeviceCmd {
    pub action: DeviceAction,
    pub service: ApiService
}
impl DeviceCmd {
    pub async fn new(action: DeviceAction, config: &str, base_url: &str, bearer_access_token: Option<String>) -> Result<Self> {
        let service = ApiService::new(config, base_url, bearer_access_token)?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String>{
        let result = match self.action {
            DeviceAction::Get => self.service.device_retrieve().await?
        };
        debug!("Success action={} result={:?}", self.action, result);
        Ok(self.service.to_string_pretty::<models::Device>(result)?)
    }    
}
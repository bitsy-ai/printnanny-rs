use anyhow::{ Result };
use clap::arg_enum;
use log:: { debug };

use services::printnanny_api::ApiService;

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
    pub async fn new(action: DeviceAction, config: &str, base_url: &str) -> Result<Self> {
        let service = ApiService::new(config, base_url).await?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String>{
        let result = match self.action {
            DeviceAction::Get => self.service.device_retrieve().await?
        };
        debug!("Success action={} result={:?}", self.action, result);
        Ok(self.service.to_string_pretty(result)?)
    }    
}
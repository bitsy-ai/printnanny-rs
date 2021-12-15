use anyhow::{ Result, anyhow };
use async_trait::async_trait;
use clap::arg_enum;
use log:: { debug };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;
use printnanny_api_client::apis::devices_api::{
    devices_retrieve,
};
use printnanny_api_client::models::{ 
    Device
};

use crate::services::generic::{ ApiService, PrintNannyService };


arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum DeviceAction{
        Get,
    }
}

#[async_trait]
impl ApiService<Device> for PrintNannyService<Device> {
    async fn retrieve(&self) -> Result<Device>{
        Ok(devices_retrieve(&self.request_config, self.api_config.device_id).await?)
    }
}

impl PrintNannyService<Device> {
}

pub async fn handle_device_cmd(action: DeviceAction, config: &str) -> Result<String>{
    let service = PrintNannyService::<Device>::new(config)?;
    let result = match action {
        DeviceAction::Get => service.retrieve().await?
    };
    debug!("Success action={} config={} result={:?}", action, config, result);
    
    Ok(service.to_string_pretty(result)?)
}
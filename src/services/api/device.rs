use anyhow::{ Result };
use async_trait::async_trait;
use clap::arg_enum;
use log:: { debug };

use printnanny_api_client::apis::devices_api::{
    devices_retrieve,
};
use printnanny_api_client::models::{ 
    Device
};

use super::generic::{ ApiService, PrintNannyService };

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum DeviceAction{
        Get,
    }
}

pub struct DeviceCmd {
    pub action: DeviceAction,
    pub service: PrintNannyService::<Device>
}

impl DeviceCmd {
    pub fn new(action: DeviceAction, config: &str) -> Result<Self> {
        let service = PrintNannyService::<Device>::new(config)?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String>{
        let result = match self.action {
            DeviceAction::Get => self.service.retrieve(self.service.license.device).await?
        };
        debug!("Success action={} result={:?}", self.action, result);
        Ok(self.service.to_string_pretty(result)?)
    }    
}


#[async_trait]
impl ApiService<Device> for PrintNannyService<Device> {
    async fn retrieve(&self, id: i32) -> Result<Device>{
        Ok(devices_retrieve(&self.request_config, id).await?)
    }
}

impl PrintNannyService<Device> {
}

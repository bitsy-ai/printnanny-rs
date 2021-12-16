use anyhow::{ Result, anyhow };
use async_trait::async_trait;
use clap::arg_enum;
use log:: { debug };

use printnanny_api_client::apis::licenses_api::{
    license_activate,
    licenses_retrieve
};
use printnanny_api_client::models::{ 
    License,
    Device,
    TaskType
};
use crate::services::generic::{ ApiService, PrintNannyService };

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum LicenseAction{
        Activate,
        Check,
        Get,
    }
}

#[async_trait]
impl ApiService<License> for PrintNannyService<License> {
    async fn retrieve(&self, id: i32) -> Result<License>{
        Ok(licenses_retrieve(&self.request_config, id).await?)
    }
}

impl PrintNannyService<License> {
    async fn activate(&self, id: i32) -> Result<License>
    {
        let service = PrintNannyService::new(&self.config)?;
        let device = service.retrieve().await?;
        Ok(license_activate(&self.request_config, id, None).await?)
    }

    fn check_task_type(&self, device: &Device, expected_type: TaskType) -> Result<()>{
        match &device.last_task {
            Some(last_task) => {
                if last_task.task_type.unwrap() != expected_type {
                    return Err(anyhow!("Expected Device.last_task to be {:?} but received task {:?}", expected_type, last_task))
                } else { Ok(()) }
            },
            None => {
                return Err(anyhow!("Expected Device.last_task to be {:?} but received task None", expected_type))
            }
        }
    }

    async fn check(&self) ->  Result<License>
    {

        match &self.item {
            Some(item) => Ok(license_activate(&self.request_config, item.id, None).await?),
            None => Err(anyhow!("PrintNannyService.item not set, but id is required for retrieve method"))
        }
    }
}

pub async fn handle_license_cmd(action: LicenseAction, config: &str) -> Result<String>{
    let mut service = PrintNannyService::<License>::new(config)?;
    service.item = Some(service.read_json(&service.paths.license_json)?);
    let result = match action {
        LicenseAction::Activate => service.activate().await?,
        LicenseAction::Get => service.retrieve().await?,
        LicenseAction::Check => service.check().await?
    };
    debug!("Success action={} config={} result.updated_dt={:?}", action, config, result);
    Ok(service.to_string_pretty(result)?)
}
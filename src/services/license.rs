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

    /// Check validity of license
    /// Mark license activated
    async fn activate(&self) -> Result<License>
    {
        self.check_license().await?;
        let device = self.retrieve(self.license.id).await?;
        Ok(license_activate(&self.request_config, self.license.id, None).await?)
    }

    fn check_task_type(&self, device: &Device, expected_type: TaskType) -> Result<()>{
        match &device.last_task {
            Some(last_task) => {
                if last_task.task_type != expected_type {
                    return Err(anyhow!("Expected Device.last_task to be {:?} but received task {:?}", expected_type, last_task))
                } else { Ok(()) }
            },
            None => {
                return Err(anyhow!("Expected Device.last_task to be {:?} but received task None", expected_type))
            }
        }
    }
}

pub async fn handle_license_cmd(action: LicenseAction, config: &str) -> Result<String>{
    let service = PrintNannyService::<License>::new(config)?;
    let result = match action {
        LicenseAction::Get => service.retrieve(service.license.id).await?,
        LicenseAction::Check => service.check_license().await?
    };
    debug!("Success action={} config={} result.updated_dt={:?}", action, config, result);
    Ok(service.to_string_pretty(result)?)
}
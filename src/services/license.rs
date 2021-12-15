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
    Device
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
    async fn retrieve(&self) -> Result<License>{
        match &self.item {
            Some(item) => Ok(licenses_retrieve(&self.request_config, item.id).await?),
            None => Err(anyhow!("PrintNannyService.item not set, but id is required for retrieve method"))
        }
    }
}

impl PrintNannyService<License> {
    async fn activate(&self) -> Result<License>
    {
        self.check().await?;
        match &self.item {
            Some(item) => Ok(license_activate(&self.request_config, item.id, None).await?),
            None => Err(anyhow!("PrintNannyService.item not set, but id is required for retrieve method"))
        }
    }

    async fn check(&self) ->  Result<License>
    {
        let service = PrintNannyService::<Device>::new(&self.config)?;
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
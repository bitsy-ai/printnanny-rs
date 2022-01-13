use anyhow::{ Result };
use clap::arg_enum;
use log:: { debug };
use printnanny_services::printnanny_api::{ ApiService };

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum LicenseAction{
        Check,
        Get,
        Generate
    }
}

pub struct LicenseCmd {
    pub action: LicenseAction,
    pub service: ApiService
}

impl LicenseCmd{
    pub async fn new(action: LicenseAction, config: &str, base_url: &str, bearer_access_token: Option<String>) -> Result<Self> {
        let service = ApiService::new(config, base_url, bearer_access_token)?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String>{
        let result = match self.action {
            LicenseAction::Get => self.service.license_retrieve_active().await?,
            LicenseAction::Check => self.service.license_check().await?,
            LicenseAction::Generate => self.service.license_download().await?
        };
        debug!("Success action={} result.updated_dt={:?}", self.action, result.updated_dt);
        Ok(self.service.to_string_pretty(result)?)
    }
}
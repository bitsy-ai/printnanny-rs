use anyhow::{ Result, anyhow, Context };
use async_trait::async_trait;
use clap::arg_enum;
use log:: { debug };

use printnanny_api_client::apis::licenses_api::{
    license_activate,
    licenses_retrieve
};
use printnanny_api_client::models::{ 
    License,
    TaskType,
    TaskStatusType
};
use printnanny_api_client::apis::devices_api::{
    devices_active_license_retrieve,
    DevicesActiveLicenseRetrieveError
};
use crate::services::msgs;
use super::generic::{ ApiService, PrintNannyService };

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum LicenseAction{
        Check,
        Get,
    }
}

pub struct LicenseCmd {
    pub action: LicenseAction,
    pub service: PrintNannyService::<License>
}

impl LicenseCmd{
    pub fn new(action: LicenseAction, config: &str) -> Result<Self> {
        let service = PrintNannyService::<License>::new(config)?;
        Ok(Self { service, action })
    }
    pub async fn handle(&self) -> Result<String>{
        let result = match self.action {
            LicenseAction::Get => self.service.retrieve(self.service.license.id).await?,
            LicenseAction::Check => self.service.check_license().await?
        };
        debug!("Success action={} result.updated_dt={:?}", self.action, result.updated_dt);
        Ok(self.service.to_string_pretty(result)?)
    }
}

#[async_trait]
impl ApiService<License> for PrintNannyService<License> {
    async fn retrieve(&self, id: i32) -> Result<License>{
        Ok(licenses_retrieve(&self.request_config, id).await?)
    }
}

impl PrintNannyService<License> {
    pub async fn activate_license(&self) -> Result<License> {
        Ok(license_activate(&self.request_config, self.license.id, None).await
        .context(format!("Failed to activate license id={}", self.license.id))?)
    }

    pub async fn retreive_active_license(&self) -> Result<License, printnanny_api_client::apis::Error<DevicesActiveLicenseRetrieveError>> {
        devices_active_license_retrieve(
            &self.request_config,
            self.license.device,
        ).await
    }
    /// Check validity of license
    /// Manage state of latest Task.task_type=CheckLicense
    pub async fn check_license(&self) -> Result<License> {
        // get active license from remote
        let active_license = self.retreive_active_license().await?;

        // handle various pending/running/failed/success states of last check task
        // return active license check task in running state
        let task = match &active_license.last_check_task {
            // check state of last task
            Some(last_check_task) => {
                match &last_check_task.last_status {
                    Some(last_status) => {
                        // assume failed state if task status can't be read
                        match last_status.status {
                            // task state is already started, no update needed
                            TaskStatusType::Started => None,
                            // task state is pending, awaiting acknowledgement from device. update to started to ack.
                            TaskStatusType::Pending => Some(self.update_task_status(last_check_task.id, TaskStatusType::Started, None, None).await?),
                            // for Failed, Success, and Timeout states create a new task
                            _ => Some(self.create_task(
                                TaskType::CheckLicense,
                                Some(TaskStatusType::Started),
                                Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
                                None
                            ).await?)
                        }
                    },
                    None => Some(self.create_task(TaskType::CheckLicense, Some(TaskStatusType::Started), None, None).await?)
                }
            },
            // no license check task found, create one in a running state
            None => Some(self.create_task(TaskType::CheckLicense, Some(TaskStatusType::Started), None, None).await?)
        };

        let task_id = match task{
            Some(t) => t.id,
            None => active_license.last_check_task.as_ref().unwrap().id
        };

        // check license ids and fingerprints
        if (self.license.id != active_license.id) || (self.license.fingerprint != active_license.fingerprint) {
            self.update_task_status(
                task_id, 
                TaskStatusType::Failed,
                Some(msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
                ).await?;
            return Err(anyhow!(
                "License mismatch local={} active={}", 
                &self.license.id, &active_license.id
            ))
        } else if active_license.activated.as_ref().unwrap() == &true {
            return Ok(active_license)
        }
        // ensure license marked activated
        else {
            let result = self.activate_license().await?;
            self.update_task_status(
                task_id, 
                TaskStatusType::Success,
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
                ).await?;
            return Ok(result)
        }
    }
}

use anyhow::{ Result, anyhow, Context };
use async_trait::async_trait;
use clap::arg_enum;
use log:: { debug, info };

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
use crate::msgs;
use super::generic::{ ApiModel, PrintNannyService };

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
impl ApiModel<License> for PrintNannyService<License> {
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
        info!("Checking validity of local license.json {}", self.license.fingerprint);
        let active_license = self.retreive_active_license().await?;
        info!("Retrieved active license for device_id={} {}", active_license.device, active_license.fingerprint);

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
                            TaskStatusType::Started => {
                                info!("Task is already in Started state, skipping update {:?}", last_check_task);
                                None
                            },
                            // task state is pending, awaiting acknowledgement from device. update to started to ack.
                            TaskStatusType::Pending => {
                                info!("Task is Pending state, sending Started status update {:?}", last_check_task);
                                Some(self.update_task_status(last_check_task.id, TaskStatusType::Started, None, None).await?)
                            },
                            // for Failed, Success, and Timeout states create a new task
                            _ => {
                                info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
                                Some(self.create_task(
                                    TaskType::SystemCheck,
                                    Some(TaskStatusType::Started),
                                    Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
                                    None
                                ).await?)
                            }
                        }
                    },
                    None => {
                        info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
                        Some(self.create_task(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
                    }
                }
            },
            // no license check task found, create one in a running state
            None => {
                info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
                Some(self.create_task(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
            }
        };

        info!("Updated task {:?}", task);


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

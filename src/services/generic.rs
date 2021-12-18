
use std::fs::{ read_to_string };
use std::path::{ PathBuf };
use log::{ info };

use anyhow::{ anyhow, Context, Result };
use async_trait::async_trait;
use serde::{ Serialize };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;
use printnanny_api_client::apis::devices_api::{
    devices_active_license_retrieve,
    devices_tasks_create,
    devices_tasks_status_create,
    DevicesTasksStatusCreateError,
    DevicesActiveLicenseRetrieveError
};
use printnanny_api_client::apis::licenses_api::{
    license_activate
};

use printnanny_api_client::models::{ 
    Device,
    License,
    TaskType,
    TaskRequest,
    TaskStatusRequest,
    TaskStatusType,
    Task
};
use crate::services::msgs;
use crate::paths::{ PrintNannyPath };

#[derive(Debug, Clone)]
pub struct PrintNannyService<T>{
    pub api_config: PrintNannyApiConfig,
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub item: Option<T>,
    pub license: License // loaded from license.json
}

fn read_model_json<T:serde::de::DeserializeOwned >(path: &PathBuf) -> Result<T> {
    let result = serde_json::from_str::<T>(
        &read_to_string(path)
        .context(format!("Failed to read {:?}", path))?
        )?;
    Ok(result)
}

impl<T> PrintNannyService<T> {
    pub fn new(config: &str) -> Result<PrintNannyService<T>> {
        let paths = PrintNannyPath::new(config);

        // api_config.json is bundled in printnanny_license.zip
        let api_config = read_model_json::<PrintNannyApiConfig>(&paths.api_config_json)?;
        
        // license.json is bundled in printnanny_license.zip
        let mut license = read_model_json::<License>(&paths.license_json)?;
        // refresh license from remote

        let request_config = Configuration{ 
            base_path: api_config.base_path.clone(),
            bearer_access_token: Some(api_config.bearer_access_token.clone()),
            ..Configuration::default()
        };

        Ok(PrintNannyService{request_config, api_config, paths, license, item: None, config: config.to_string() })
    }

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

    pub async fn update_task_status(
        &self, 
        task_id: i32,
        status: TaskStatusType,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<Task, printnanny_api_client::apis::Error<DevicesTasksStatusCreateError>> {
        let request = TaskStatusRequest{detail, wiki_url, task: task_id, status};
        info!("Submitting TaskStatusRequest={:?}", request);
        return devices_tasks_status_create(
            &self.request_config,
            self.license.device,
            task_id,
            request
        ).await
    }

    pub async fn create_task(&self, task_type: TaskType, status: Option<TaskStatusType>, detail: Option<String>, wiki_url: Option<String>) -> Result<Task> {
        let request = TaskRequest{
            active: Some(true),
            task_type: task_type,
            device: self.license.device
        };
        let task = devices_tasks_create(&self.request_config, self.license.device, request).await?;
        info!("Created task={:?}", task);
        let task = match status {
            Some(s) => self.update_task_status(task.id, s, wiki_url, detail, ).await?,
            None => task
        };
        Ok(task)
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

#[async_trait]
pub trait ApiService<T:serde::de::DeserializeOwned + Serialize> {
    // async fn create<T, R>(&self, request: R) -> Result<T>;
    async fn retrieve(&self, id: i32) -> Result<T>;
    // async fn partial_update<T, R>(&self, id: &i32, rquest: R) -> Result<T>;
    // async fn update<T, R>(&self, id: &i32, request: R) -> Result<T>;

    fn read_json(&self, path: &PathBuf) -> Result<T> {
        let result = serde_json::from_str::<T>(
            &read_to_string(path)
            .context(format!("Failed to read {:?}", path))?
            )?;
        Ok(result)
    }
    fn to_string_pretty(&self, item: T) -> Result<String> {
        Ok(serde_json::to_string_pretty::<T>(&item)?)
    }

    fn check_task_type(&self, device: &Device, expected_type: TaskType) -> Result<()>{
        match &device.last_task {
            Some(last_task) => {
                if last_task.task_type == expected_type {
                    return Err(anyhow!("Expected Device.last_task to be {:?} but received task {:?}", expected_type, last_task))
                } else { Ok(()) }
            },
            None => {
                return Err(anyhow!("Expected Device.last_task to be {:?} but received task None", expected_type))
            }
        }
    }
}

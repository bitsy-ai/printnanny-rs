
use std::fs::{ read_to_string };
use std::path::{ PathBuf };
use log::{ info, warn, error };

use anyhow::{ anyhow, Context, Result };

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;

use printnanny_api_client::apis::auth_api::{
    auth_email_create
};
use printnanny_api_client::apis::devices_api::{
    devices_tasks_create,
    devices_tasks_status_create,
};
use printnanny_api_client::apis::licenses_api::{
    license_activate,
    licenses_retrieve
};
use printnanny_api_client::apis::devices_api::{
    devices_active_license_retrieve,
};
use printnanny_api_client::models::{ 
    Device,
    License,
    TaskType,
    TaskRequest,
    TaskStatusRequest,
    TaskStatusType,
    Task,
    EmailAuthRequest,
    DetailResponse
};
use printnanny_api_client::apis::devices_api::{
    devices_retrieve,
};
use crate::paths::{ PrintNannyPath };
use crate::msgs;

#[derive(Debug, Clone)]
pub struct ApiService{
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub license: Option<License>,
    pub device: Option<Device>
}

fn read_model_json<T:serde::de::DeserializeOwned >(path: &PathBuf) -> Result<T> {
    let result = serde_json::from_str::<T>(
        &read_to_string(path)
        .context(format!("Failed to read {:?}", path))?
        )?;
    Ok(result)
}

impl ApiService {
    pub async fn new(config: &str, base_url: &str) -> Result<ApiService> {
        let paths = PrintNannyPath::new(config);

        // api_config.json cached to /opt/printnanny/data
        let read_api_config = read_model_json::<PrintNannyApiConfig>(&paths.api_config_json);
        let request_config = match read_api_config {
            Ok(api_config) => {
                Configuration{ 
                    base_path: api_config.base_path.clone(),
                    bearer_access_token: Some(api_config.bearer_access_token.clone()),
                    ..Configuration::default()
                }
            },
            Err(e) => {
                warn!("Failed to read {:?} - calling api as anonymous user", &paths.api_config_json);
                error!("{}", e);
                Configuration{ 
                    base_path: base_url.to_string(),
                    ..Configuration::default()
                }
            }
        };

        // device.json cached to /opt/printnanny/data
        let read_device_json = read_model_json::<Device>(&paths.device_json);
        let device: Option<Device> = match read_device_json {
            Ok(device) => Some(device),
            Err(e) => {
                warn!("Failed to read {:?} - methods requiring device will fail", &paths.device_json);
                error!("{}", e);
                None
            }
        };

        // license.json cached to /opt/printnanny/data
        let read_license_json = read_model_json::<License>(&paths.license_json);
        let license: Option<License> = match read_license_json {
            Ok(license) => Some(license),
            Err(e) => {
                warn!("Failed to read {:?} - methods requiring license will fail", &paths.license_json);
                error!("{}", e);
                None
            }
        };


        Ok(Self{
            request_config, 
            paths, 
            config: config.to_string(),
            device: device,
            license: license
        })
    }
    // auth APIs
    pub async fn auth_email_create(&self, email: String) -> Result<DetailResponse> {
        let req = EmailAuthRequest{email};
        let res = auth_email_create(&self.request_config, req).await?;
        Ok(res)
    }
    // device API
    pub async fn device_retrieve(&self) -> Result<Device> {
        match &self.device {
            Some(device) => {
                Ok(devices_retrieve(&self.request_config, device.id).await?)
            },
            None => Err(anyhow!("ApiService.device_retrieve called without ApiService.device set"))
        }
    }
    // license API
    pub async fn license_activate(&self) -> Result<License> {
        match &self.license {
            Some(license) => {
                Ok(license_activate(&self.request_config, license.id, None).await
                .context(format!("Failed to activate license id={}", license.id))?)
            },
            None => Err(anyhow!("ApiService.license_activate called without ApiService.license set"))
        }
    }
    pub async fn license_retrieve(&self) -> Result<License> {
        match &self.license {
            Some(license) => {
                Ok(licenses_retrieve(&self.request_config, license.id).await
                .context(format!("Failed to activate license id={}", license.id))?)
            },
            None => Err(anyhow!("ApiService.license_activate called without ApiService.license set"))
        }
    }
    pub async fn license_retreive_active(&self) -> Result<License> {
        match &self.device {
            Some(device) => {
                Ok(devices_active_license_retrieve(
                    &self.request_config,
                    device.id,
                ).await?)
            },
            None => Err(anyhow!("ApiService.license_retreive_active called without ApiService.device set"))
        }
    }
    pub async fn license_check(&self) -> Result<License> {
        match &self.license {
            Some(license) => {
                // get active license from remote
                info!("Checking validity of local license.json {}", license.fingerprint);
                let active_license = self.license_retreive_active().await?;
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
                                        Some(self.task_status_create(last_check_task.id, TaskStatusType::Started, None, None).await?)
                                    },
                                    // for Failed, Success, and Timeout states create a new task
                                    _ => {
                                        info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
                                        Some(self.task_create(
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
                                Some(self.task_create(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
                            }
                        }
                    },
                    // no license check task found, create one in a running state
                    None => {
                        info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
                        Some(self.task_create(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
                    }
                };

                info!("Updated task {:?}", task);


                let task_id = match task{
                    Some(t) => t.id,
                    None => active_license.last_check_task.as_ref().unwrap().id
                };

                // check license ids and fingerprints
                if (license.id != active_license.id) || (license.fingerprint != active_license.fingerprint) {
                    self.task_status_create(
                        task_id, 
                        TaskStatusType::Failed,
                        Some(msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string()),
                        Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
                        ).await?;
                    return Err(anyhow!(
                        "License mismatch local={} active={}", 
                        license.id, &active_license.id
                    ))
                }
                // ensure license marked activated
                else {
                    let result = self.license_activate().await?;
                    self.task_status_create(
                        task_id, 
                        TaskStatusType::Success,
                        Some(msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string()),
                        Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
                        ).await?;
                    return Ok(result)
                }
            },
            None => Err(anyhow!("ApiService.license_retreive_active called without ApiService.device set"))
        }
    }
    // task status API
    pub async fn task_status_create(
        &self, 
        task_id: i32,
        status: TaskStatusType,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<Task> {
        match &self.device {
            Some(device) => {
                let request = TaskStatusRequest{detail, wiki_url, task: task_id, status};
                info!("Submitting TaskStatusRequest={:?}", request);
                Ok(devices_tasks_status_create(
                    &self.request_config,
                    device.id,
                    task_id,
                    request
                ).await?)
            },
            None => Err(anyhow!("ApiService.task_status_create called without ApiService.device set"))
        }
    }

    pub async fn task_create(&self, task_type: TaskType, status: Option<TaskStatusType>, detail: Option<String>, wiki_url: Option<String>) -> Result<Task> {
        match &self.device {
            Some(device) => {
                let request = TaskRequest{
                    active: Some(true),
                    task_type: task_type,
                    device: device.id
                };
                let task = devices_tasks_create(&self.request_config, device.id, request).await?;
                info!("Created task={:?}", task);
                let task = match status {
                    Some(s) => self.task_status_create(task.id, s, wiki_url, detail, ).await?,
                    None => task
                };
                Ok(task)
            },
            None => Err(anyhow!("ApiService.task_create called without ApiService.device set"))
        }
    }
    pub fn to_string_pretty<T: serde::Serialize>(&self, item: T) -> Result<String> {
        Ok(serde_json::to_string_pretty::<T>(&item)?)
    }
}

// #[async_trait]
// pub trait ApiModel<T:serde::de::DeserializeOwned + Serialize> {
//     // async fn create<T, R>(&self, request: R) -> Result<T>;
//     async fn retrieve(&self, id: i32) -> Result<T>;
//     // async fn partial_update<T, R>(&self, id: &i32, rquest: R) -> Result<T>;
//     // async fn update<T, R>(&self, id: &i32, request: R) -> Result<T>;

//     fn read_json(path: &PathBuf) -> Result<T> {
//         return read_model_json::<T>(path)
//     }

//     fn to_string_pretty(&self, item: T) -> Result<String> {
//         Ok(serde_json::to_string_pretty::<T>(&item)?)
//     }
// }

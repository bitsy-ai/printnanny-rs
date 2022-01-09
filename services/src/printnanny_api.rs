use std::fs::{ File };
use std::io::BufReader;
use std::path::{ PathBuf };
use log::{ info, warn, error };

use thiserror::Error;
use serde::{Serialize, Deserialize};

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;

use printnanny_api_client::apis::Error as ApiError;
use printnanny_api_client::apis::auth_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::licenses_api;
use printnanny_api_client::models;

use crate::paths::{ PrintNannyPath };
use crate::msgs;

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardCookie {
    api_config: models::PrintNannyApiConfig,
    user: models::User,
    device: models::Device,
    analytics: bool,
}

#[derive(Error, Debug)]
pub enum ServiceError<T>{
    #[error(transparent)]
    ApiError(#[from] ApiError<T>),
    #[error("License fingerprint mismatch (expected {expected:?}, found {active:?})")]
    InvalidLicense {
        expected: String,
        active: String,
    },
    #[error(transparent)]
    SysInfoError(#[from] sys_info::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("Signup incomplete - failed to read from {cache:?}")]
    SignupIncomplete{
        cache: PathBuf
    }
}

#[derive(Debug, Clone)]
pub struct ApiService{
    pub request_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub license: Option<models::License>,
    pub device: Option<models::Device>
}

fn read_model_json<T:serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let result: T = serde_json::from_reader(reader)?;
    Ok(result)
}

fn save_model_json<T:serde::Serialize>(model: &T, path: &PathBuf) -> Result<(),  std::io::Error> {
    serde_json::to_writer(&File::create(path)?, model)?;
    Ok(())
}

impl ApiService {
    pub async fn new(config: &str, base_url: &str) -> Result<ApiService, ServiceError<()>> {
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
                Configuration{ 
                    base_path: base_url.to_string(),
                    ..Configuration::default()
                }
            }
        };

        // attempt to cache models to /opt/printnanny/data
        let mut s = Self{
            request_config,
            paths, 
            config: config.to_string(),
            device: None,
            license: None
        };
        s.load_models().await?;
        Ok(s)
    }

    pub async fn load_models(&mut self) -> Result<(), ServiceError<()>>{
        let device = self.load_device_json().await;
        match device {
            Ok(v) => {
                self.device = Some(v);
            },
            Err(e) =>{
                error!("Failed to load device.json {:?}", e);
                self.device = None;
            }
        };
        let license = self.load_license_json().await;
        match license {
            Ok(v) => {
                self.license = Some(v);
            },
            Err(e) => {
                error!("Failed to load license.json {:?}", e);
                self.license = None;
            }
        };
        Ok(())
    }

    // auth APIs
    pub async fn auth_email_create(&self, email: String) -> Result<models::DetailResponse, ApiError::<auth_api::AuthEmailCreateError>> {
        let req = models::EmailAuthRequest{email};
        auth_api::auth_email_create(&self.request_config, req).await
    }
    pub async fn auth_token_validate(&self, email: &str, token: &str) -> Result<models::TokenResponse, ApiError::<auth_api::AuthTokenCreateError>> {
        let req = models::CallbackTokenAuthRequest{email: Some(email.to_string()), token: token.to_string(), mobile: None};
        auth_api::auth_token_create(&self.request_config, req).await
    }
    // device API
    pub async fn device_retrieve(&self) -> Result<models::Device, ServiceError<devices_api::DevicesRetrieveError>> {
        match &self.device {
            Some(device) => Ok(devices_api::devices_retrieve(&self.request_config, device.id).await?),
            None => Err(ServiceError::SignupIncomplete{cache: self.paths.device_json.clone() })
        }
    }
    pub async fn device_retrieve_hostname(&self) -> Result<models::Device, ServiceError<devices_api::DevicesRetrieveHostnameError>> {
        let hostname = sys_info::hostname()?;
        let res = devices_api::devices_retrieve_hostname(&self.request_config, &hostname).await?;
        Ok(res)
    }
    // license API
    pub async fn license_activate(&self, license_id: i32) -> Result<models::License, ApiError<licenses_api::LicenseActivateError>> {
        licenses_api::license_activate(&self.request_config, license_id, None).await
    }
    pub async fn license_retrieve(&self, license_id: i32) -> Result<models::License, ApiError<licenses_api::LicensesRetrieveError>> {
        licenses_api::licenses_retrieve(&self.request_config, license_id).await
    }
    pub async fn license_retreive_active(&self) -> Result<models::License, ServiceError<devices_api::DevicesActiveLicenseRetrieveError>> {
        match &self.device {
            Some(device) => Ok(devices_api::devices_active_license_retrieve(
                &self.request_config,
                device.id,
            ).await?),
            None => Err(ServiceError::SignupIncomplete{cache: self.paths.device_json.clone() })
        }
    }

    pub async fn license_check(&self) -> Result<(), ServiceError<()>> {
        // read license from list
        // create task
        // check license
        // update task
        Ok(())
    }

    // read device.json from disk cache @ /var/run/printnanny
    // hydrate cache if device.json not found
    pub async fn load_device_json(&self) -> Result<models::Device, ServiceError<devices_api::DevicesRetrieveHostnameError>> {
        let m = read_model_json::<models::Device>(&self.paths.device_json);
        match m {
            Ok(device) => Ok(device),
            Err(e) => {
                warn!("Failed to read {:?} - attempting to load device.json from remote", &self.paths.device_json);
                let res = self.device_retrieve_hostname().await;
                match res {
                    Ok(device) => {
                        save_model_json::<models::Device>(&device, &self.paths.device_json)?;
                        info!("Saved model {:?} to {:?}", &device, &self.paths.device_json);
                        Ok(device)
                    }
                    Err(e) => Err(e)
                }
            }
        }
    }

    // read license.json from disk cache @ /var/run/printnanny
    // hydrate cache if license.json not found
    pub async fn load_license_json(&self) -> Result<models::License, ServiceError<devices_api::DevicesActiveLicenseRetrieveError>> {
        let m = read_model_json::<models::License>(&self.paths.license_json);
        match m {
            Ok(license) => Ok(license),
            Err(e) => {
                warn!("Failed to read {:?} - attempting to load license.json from remote", &self.paths.license_json);
                let license = self.license_retreive_active().await?;
                save_model_json::<models::License>(&license, &self.paths.license_json)?;
                info!("Saved model {:?} to {:?}", &license, &self.paths.license_json);
                Ok(license)
            }
        }
    }
    // pub async fn license_check(&self, license: &License) -> Result<License, ServiceError::InvalidLicense> {
    //     match &self.license {
    //         Some(license) => {
    //             // get active license from remote
    //             info!("Checking validity of local license.json {}", license.fingerprint);
    //             let active_license = self.license_retreive_active().await?;
    //             info!("Retrieved active license for device_id={} {}", active_license.device, active_license.fingerprint);

    //             // handle various pending/running/failed/success states of last check task
    //             // return active license check task in running state
    //             let task = match &active_license.last_check_task {
    //                 // check state of last task
    //                 Some(last_check_task) => {
    //                     match &last_check_task.last_status {
    //                         Some(last_status) => {
    //                             // assume failed state if task status can't be read
    //                             match last_status.status {
    //                                 // task state is already started, no update needed
    //                                 TaskStatusType::Started => {
    //                                     info!("Task is already in Started state, skipping update {:?}", last_check_task);
    //                                     None
    //                                 },
    //                                 // task state is pending, awaiting acknowledgement from device. update to started to ack.
    //                                 TaskStatusType::Pending => {
    //                                     info!("Task is Pending state, sending Started status update {:?}", last_check_task);
    //                                     Some(self.task_status_create(last_check_task.id, TaskStatusType::Started, None, None).await?)
    //                                 },
    //                                 // for Failed, Success, and Timeout states create a new task
    //                                 _ => {
    //                                     info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
    //                                     Some(self.task_create(
    //                                         TaskType::SystemCheck,
    //                                         Some(TaskStatusType::Started),
    //                                         Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
    //                                         None
    //                                     ).await?)
    //                                 }
    //                             }
    //                         },
    //                         None => {
    //                             info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
    //                             Some(self.task_create(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
    //                         }
    //                     }
    //                 },
    //                 // no license check task found, create one in a running state
    //                 None => {
    //                     info!("No active task found, creating task {:?} ", TaskType::SystemCheck);
    //                     Some(self.task_create(TaskType::SystemCheck, Some(TaskStatusType::Started), None, None).await?)
    //                 }
    //             };

    //             info!("Updated task {:?}", task);


    //             let task_id = match task{
    //                 Some(t) => t.id,
    //                 None => active_license.last_check_task.as_ref().unwrap().id
    //             };

    //             // check license ids and fingerprints
    //             if (license.id != active_license.id) || (license.fingerprint != active_license.fingerprint) {
    //                 self.task_status_create(
    //                     task_id, 
    //                     TaskStatusType::Failed,
    //                     Some(msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string()),
    //                     Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
    //                     ).await?;
    //                 return Err(anyhow!(
    //                     "License mismatch local={} active={}", 
    //                     license.id, &active_license.id
    //                 ))
    //             }
    //             // ensure license marked activated
    //             else {
    //                 let result = self.license_activate().await?;
    //                 self.task_status_create(
    //                     task_id, 
    //                     TaskStatusType::Success,
    //                     Some(msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string()),
    //                     Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
    //                     ).await?;
    //                 return Ok(result)
    //             }
    //         },
    //         None => Err(anyhow!("ApiService.license_retreive_active called without ApiService.device set"))
    //     }
    // }
    // task status API

    // pub async fn task_status_create(
    //     &self, 
    //     task_id: i32,
    //     device_id: i32,
    //     status: TaskStatusType,
    //     detail: Option<String>,
    //     wiki_url: Option<String>,
    // ) -> Result<Task, Box<dyn std::error::Error>> {

    //     let request = TaskStatusRequest{detail, wiki_url, task: task_id, status};
    //     info!("Submitting TaskStatusRequest={:?}", request);
    //     Ok(devices_tasks_status_create(
    //         &self.request_config,
    //         device_id,
    //         task_id,
    //         request
    //     ).await
    // }

    // pub async fn task_create(
    //     &self, 
    //     task_type: TaskType, 
    //     status: Option<TaskStatusType>,
    //     detail: Option<String>,
    //     wiki_url: Option<String>
    // ) -> Result<Task, ServiceError<devices_api::DevicesTasksCreateError>> {
    //     match &self.device {
    //         Some(device) => {
    //             let request = TaskRequest{
    //                 active: Some(true),
    //                 task_type: task_type,
    //                 device: device.id
    //             };
    //             let task = devices_tasks_create(&self.request_config, device.id, request).await?;
    //             info!("Created task={:?}", task);
    //             let task = match status {
    //                 Some(s) => self.task_status_create(task.id, s, wiki_url, detail, ).await?,
    //                 None => task
    //             };
    //             Ok(task)
    //         },
    //         None => Err(anyhow!("ApiService.task_create called without ApiService.device set"))
    //     }
    // }
    // pub fn to_string_pretty<T: serde::Serialize>(&self, item: T) -> Result<String> {
    //     Ok(serde_json::to_string_pretty::<T>(&item)?)
    // }
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

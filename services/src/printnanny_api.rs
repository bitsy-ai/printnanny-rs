use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{ PathBuf };
use std::future::Future;
use std::process::Command;
use log::{ info, warn, error };

use thiserror::Error;
use serde::{Serialize, Deserialize};

use printnanny_api_client::models::print_nanny_api_config::PrintNannyApiConfig;
use printnanny_api_client::apis::configuration::Configuration;

use printnanny_api_client::apis::Error as ApiError;
use printnanny_api_client::apis::auth_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::licenses_api;
use printnanny_api_client::apis::users_api;
use printnanny_api_client::models;

use crate::paths::{ PrintNannyPath };
use crate::msgs;

#[derive(Error, Debug)]
pub enum ServiceError{
    #[error(transparent)]
    AuthTokenCreateError(#[from] ApiError<auth_api::AuthTokenCreateError>),
    #[error(transparent)]
    AuthEmailCreateError(#[from] ApiError<auth_api::AuthEmailCreateError>),

    #[error(transparent)]
    DevicesCreateError(#[from] ApiError<devices_api::DevicesCreateError>),

    #[error(transparent)]
    DevicesRetrieveError(#[from] ApiError<devices_api::DevicesRetrieveError>),
    #[error(transparent)]
    DevicesGenerateLicenseError(#[from] ApiError<devices_api::DevicesGenerateLicenseError>),

    #[error(transparent)]
    LicenseActivate(#[from] ApiError<licenses_api::LicenseActivateError>),
    
    #[error(transparent)]
    DevicesActiveLicenseRetrieveError(#[from] ApiError<devices_api::DevicesActiveLicenseRetrieveError>),

    #[error(transparent)]
    DevicesRetrieveHostnameError(#[from] ApiError<devices_api::DevicesRetrieveHostnameError>),

    #[error(transparent)]
    TaskCreateError(#[from] ApiError<devices_api::DevicesTasksCreateError>),

    #[error(transparent)]
    TaskStatusCreateError(#[from] ApiError<devices_api::DevicesTasksStatusCreateError>),

    #[error(transparent)]
    UsersRetrieveError(#[from] ApiError<users_api::UsersMeRetrieveError>),


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
    },
    #[error("Missing Print Nanny API token")]
    SetupIncomplete{}
}

#[derive(Debug, Clone)]
pub struct ApiService{
    pub reqwest_config: Configuration,
    pub paths: PrintNannyPath,
    pub config: String,
    pub license: Option<models::License>,
    pub device: Option<models::Device>,
    pub user: Option<models::User>
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
    // config priority:
    // args >> api_config.json >> anonymous api usage only
    pub fn new(config: &str, base_url: &str, bearer_access_token: Option<String>) -> Result<ApiService, ServiceError> {
        let paths = PrintNannyPath::new(config);

        // api_config.json cached to /opt/printnanny/data
        let reqwest_config = ApiService::to_reqwest_config(base_url, bearer_access_token, &paths.api_config_json);

        // attempt to cache models to /opt/printnanny/data
        Ok(Self{
            reqwest_config,
            paths, 
            config: config.to_string(),
            device: None,
            license: None,
            user: None
        })
    }

    pub fn to_api_config(&self) -> Result<models::PrintNannyApiConfig, ServiceError> {
        if let Some(bearer_access_token) = &self.reqwest_config.bearer_access_token {
            let base_path = &self.reqwest_config.base_path;
            Ok(PrintNannyApiConfig{
                base_path: base_path.to_string(),
                bearer_access_token: bearer_access_token.to_string(),
            })
        } else {
            Err(ServiceError::SetupIncomplete{})
        }
    }

    fn to_reqwest_auth_config(base_path: String, bearer_access_token: String) -> Configuration {
        Configuration{ 
            base_path,
            bearer_access_token: Some(bearer_access_token),
            ..Configuration::default()
        }
    }

    fn to_reqwest_anon_config(base_path: String) -> Configuration {
        Configuration{ 
            base_path,
            ..Configuration::default()
        }
    }

    // config priority:
    // args >> api_config.json >> anonymous api usage only
    fn to_reqwest_config(base_url: &str, bearer_access_token: Option<String>, api_config_json: &PathBuf) -> Configuration {
        // prefer token passed on command line (if present)
        match bearer_access_token {
            Some(token) => {
                info!("Authenticating with --api-token");
                ApiService::to_reqwest_auth_config(base_url.to_string(), token)
            },
            None => {
                // fall back to api_config.json (if present)
                let read_api_config = read_model_json::<PrintNannyApiConfig>(&api_config_json);
                match read_api_config {
                    Ok(api_config) => {
                        info!("Authenticating with {:?}", &api_config_json);
                        ApiService::to_reqwest_auth_config(api_config.base_path, api_config.bearer_access_token)
                    }
                    Err(_e) => {
                        warn!("Failed to read {:?} - calling api as anonymous user", &api_config_json);
                        ApiService::to_reqwest_anon_config(base_url.to_string())
                    }
                }
            }
        }
    }

    pub fn api_config_save(&self, bearer_access_token: &str) -> Result<(), ServiceError>{
        let base_path = self.reqwest_config.base_path.to_string();
        let config = models::PrintNannyApiConfig{ bearer_access_token: bearer_access_token.to_string(), base_path};
        save_model_json::<models::PrintNannyApiConfig>(&config, &self.paths.api_config_json)?;
        Ok(())
    }

    pub async fn load_models(&mut self) -> Result<(), ServiceError>{

        // load user from /me response
        let user: models::User = self.load_model(&self.paths.user_json, ApiService::auth_user_retreive(&self)).await?;
        self.user = Some(user);

        // load device by hostname
        let hostname = sys_info::hostname()?;
        let device_path = &self.paths.device_info_json;
        let device: models::Device = self.load_model(device_path, ApiService::device_retrieve_or_create_hostname(&self)).await?;
        self.device = Some(device);

        // load active license for device
        let license: models::License = self.load_license_json().await?;
        self.license = Some(license);
        Ok(())
    }

    // auth APIs

    // fetch user associated with auth token
    pub async fn auth_user_retreive(&self) -> Result<models::User, ServiceError>{
        Ok(users_api::users_me_retrieve(&self.reqwest_config).await?)
    }

    pub async fn auth_email_create(&self, email: String) -> Result<models::DetailResponse,  ServiceError> {
        let req = models::EmailAuthRequest{email};
        Ok(auth_api::auth_email_create(&self.reqwest_config, req).await?)
    }
    pub async fn auth_token_validate(&self, email: &str, token: &str) -> Result<models::TokenResponse,  ServiceError> {
        let req = models::CallbackTokenAuthRequest{email: Some(email.to_string()), token: token.to_string(), mobile: None};
        Ok(auth_api::auth_token_create(&self.reqwest_config, req).await?)
    }

    // device API

    pub async fn device_create(&self, hostname: String) -> Result<models::Device, ServiceError> {
        let req = models::DeviceRequest{
            hostname: Some(hostname),
            monitoring_active: Some(false),
            release_channel: None
        };
        Ok(devices_api::devices_create(&self.reqwest_config,req).await?)
    }

    pub async fn device_retrieve(&self) -> Result<models::Device,  ServiceError> {
        match &self.device {
            Some(device) => Ok(devices_api::devices_retrieve(&self.reqwest_config, device.id).await?),
            None => Err(ServiceError::SignupIncomplete{cache: self.paths.device_json.clone() })
        }
    }
    pub async fn device_retrieve_hostname(&self) -> Result<models::Device, ServiceError> {
        let hostname = sys_info::hostname()?;
        let res = devices_api::devices_retrieve_hostname(&self.reqwest_config, &hostname).await?;
        Ok(res)
    }

    pub async fn device_retrieve_or_create_hostname(&self) -> Result<models::Device, ServiceError>{
        let hostname = sys_info::hostname()?;
        let res = self.device_retrieve_hostname().await;
        match res {
            Ok(device) => Ok(device),
            // handle 404 / Not Found error by attempting to create device with hostname
            Err(e) => match &e {
                ServiceError::DevicesRetrieveHostnameError(ApiError::ResponseError(content)) => match content.status {
                    reqwest::StatusCode::NOT_FOUND => {
                        warn!("Failed retreive device with hostname={} error={:?} - attempting to create device", hostname, e);
                        let res = self.device_create(hostname.to_string()).await?;
                        info!("Success! Created device={:?}", res);
                        Ok(res)
                    },
                    _ => Err(e)
                },
                _ => Err(e)
            }
        }
    }

    // license API
    pub async fn license_activate(&self, license_id: i32) -> Result<models::License,  ServiceError> {
        Ok(licenses_api::license_activate(&self.reqwest_config, license_id, None).await?)
    }
    pub async fn license_retrieve_active(&self) -> Result<models::License, ServiceError> {
        match &self.device {
            Some(device) => Ok(devices_api::devices_active_license_retrieve(
                &self.reqwest_config,
                device.id,
            ).await?),
            None => Err(ServiceError::SignupIncomplete{cache: self.paths.device_json.clone() })
        }
    }

    // read <models::<T>>.json from disk cache @ /var/run/printnanny
    // hydrate cache if not found using fallback fn f (must return a Future)
    pub async fn load_model<T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug>(&self, path: &PathBuf, f: impl Future<Output = Result<T, ServiceError>>) -> Result<T, ServiceError> {
        let m = read_model_json::<T>(path);
        match m {
            Ok(v) => Ok(v),
            Err(_e) => {
                warn!("Failed to read {:?} - falling back to load remote model", path);
                let res = f.await;
                match res {
                    Ok(v) => {
                        save_model_json::<T>(&v, path)?;
                        info!("Saved model {:?} to {:?}", &v, path);
                        Ok(v)
                    }
                    Err(e) => Err(e)
                }
            }
        }
    }

    // read device.json from disk cache @ /var/run/printnanny
    // hydrate cache if device.json not found
    pub async fn load_device_json(&self) -> Result<models::Device, ServiceError> {
        let m = read_model_json::<models::Device>(&self.paths.device_json);
        match m {
            Ok(device) => Ok(device),
            Err(_e) => {
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
    pub async fn load_license_json(&self) -> Result<models::License, ServiceError> {
        let m = read_model_json::<models::License>(&self.paths.license_json);
        match m {
            Ok(license) => {
                info!("Loaded license.json from cache fingerprint={}", license.fingerprint);
                Ok(license)
            },
            Err(_e) => {
                warn!("Failed to read {:?} - attempting to load license.json from remote", &self.paths.license_json);
                let license = self.license_retrieve_active().await?;
                save_model_json::<models::License>(&license, &self.paths.license_json)?;
                info!("Saved model {:?} to {:?}", &license, &self.paths.license_json);
                Ok(license)
            }
        }
    }

    pub async fn license_download(&self) -> Result<models::License, ServiceError> {
        // download license.zip
        let device = self.device_retrieve_or_create_hostname().await?;
        info!("Got device={:?}", device);
        let license_zip_bytes = devices_api::devices_generate_license(
            &self.reqwest_config,
            device.id
        ).await?;

        // ensure data cache exists
        std::fs::create_dir_all(&self.paths.data)?;
        // write license.zip to disk
        let mut f = File::create(&self.paths.license_zip)?;
        f.write_all(&license_zip_bytes)?;
        info!("Downloaded license.zip to {:?}", &self.paths.license_zip);

        // unarchive
        let target = self.paths.license_zip.clone().into_os_string().into_string().unwrap();
        let cmd = Command::new("unzip")
            .current_dir(&self.paths.data)
            .args(["-o", &target])
            .output()
            .expect("Failed to unzip license");
        info!("unzip: {:?}", cmd);
        // load license.json
        let license = self.load_license_json().await?;
        Ok(license)
    }

    // ensure cached license.json matches active license on remote
    pub async fn license_check(&self) -> Result<models::License, ServiceError> {
        let task = self.task_create(models::TaskType::SystemCheck, Some(models::TaskStatusType::Started), None, None).await?;
        let license = self.load_license_json().await?;
        let active_license = self.license_retrieve_active().await?;
        info!("Retrieved active license for device_id={} {}", active_license.device, active_license.fingerprint);

        Ok(active_license)
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

    pub async fn task_status_create(
        &self, 
        task_id: i32,
        device_id: i32,
        status: models::TaskStatusType,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<models::TaskStatus, ServiceError> {

        let request = models::TaskStatusRequest{detail, wiki_url, task: task_id, status};
        info!("Submitting TaskStatusRequest={:?}", request);
        let res = devices_api::devices_tasks_status_create(
            &self.reqwest_config,
            device_id,
            task_id,
            request
        ).await?;
        Ok(res)
    }

    pub async fn task_create(
        &self, 
        task_type: models::TaskType, 
        status: Option<models::TaskStatusType>,
        detail: Option<String>,
        wiki_url: Option<String>
    ) -> Result<models::Task, ServiceError> {
        match &self.device {
            Some(device) => {
                let request = models::TaskRequest{
                    active: Some(true),
                    task_type: task_type,
                    device: device.id
                };
                let task = devices_api::devices_tasks_create(&self.reqwest_config, device.id, request).await?;
                info!("Success: created task={:?}", task);
                match status {
                    Some(s) => {
                        let res  = self.task_status_create(task.id, device.id, s, wiki_url, detail ).await?;
                        info!("Success: created task status={:?}", res);
                        Ok(task)
                    },
                    None => Ok(task)
                }
            },
            None => Err(ServiceError::SignupIncomplete{ cache: self.paths.device_json.clone() })
        }
    }
    pub fn to_string_pretty<T: serde::Serialize>(&self, item: T) -> serde_json::error::Result<String> {
        Ok(serde_json::to_string_pretty::<T>(&item)?)
    }
}

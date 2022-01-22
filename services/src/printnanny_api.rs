use log::{debug, error, info, warn};
use std::convert::TryInto;
use std::fs::{read_to_string, File};
use std::future::Future;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;

use printnanny_api_client::apis::auth_api;
use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::users_api;
use printnanny_api_client::apis::Error as ApiError;
use printnanny_api_client::models;
use thiserror::Error;

use crate::config::PrintNannyConfig;
use crate::cpuinfo::RpiCpuInfo;
use crate::paths::PrintNannyPath;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    AuthTokenCreateError(#[from] ApiError<auth_api::AuthTokenCreateError>),
    #[error(transparent)]
    AuthEmailCreateError(#[from] ApiError<auth_api::AuthEmailCreateError>),

    #[error(transparent)]
    DevicesCreateError(#[from] ApiError<devices_api::DevicesCreateError>),

    #[error(transparent)]
    DevicesRetrieveError(#[from] ApiError<devices_api::DevicesRetrieveError>),

    #[error(transparent)]
    DevicesRetrieveHostnameError(#[from] ApiError<devices_api::DevicesRetrieveHostnameError>),

    #[error(transparent)]
    JanusAuthCreateError(#[from] ApiError<devices_api::DevicesJanusCreateError>),

    #[error(transparent)]
    SystemInfoCreateError(#[from] ApiError<devices_api::DevicesSystemInfoCreateError>),
    #[error(transparent)]
    SystemInfoUpdateOrCreateError(#[from] ApiError<devices_api::SystemInfoUpdateOrCreateError>),

    #[error(transparent)]
    PublicKeyUpdateOrCreate(#[from] ApiError<devices_api::PublicKeyUpdateOrCreateError>),

    #[error(transparent)]
    JanusAuthUpdateOrCreate(#[from] ApiError<devices_api::JanusAuthUpdateOrCreateError>),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    TaskCreateError(#[from] ApiError<devices_api::DevicesTasksCreateError>),

    #[error(transparent)]
    TaskStatusCreateError(#[from] ApiError<devices_api::DevicesTasksStatusCreateError>),

    #[error(transparent)]
    UsersRetrieveError(#[from] ApiError<users_api::UsersMeRetrieveError>),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("License fingerprint mismatch (expected {expected:?}, found {active:?})")]
    InvalidLicense { expected: String, active: String },

    #[error("Failed to fingerprint {path:?} got stderr {stderr:?}")]
    FingerprintError { path: PathBuf, stderr: String },

    #[error(transparent)]
    FigmentError(#[from] procfs::ProcError),

    #[error(transparent)]
    SysInfoError(#[from] sys_info::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("Signup incomplete - failed to read from {cache:?}")]
    SignupIncomplete { cache: PathBuf },
    #[error("Missing Print Nanny API token")]
    SetupIncomplete {},
}

#[derive(Debug, Clone)]
pub struct ApiService {
    pub reqwest: ReqwestConfig,
    pub paths: PrintNannyPath,
    pub config: PrintNannyConfig,
    pub device: Option<models::Device>,
    pub user: Option<models::User>,
}

pub fn read_model_json<T: serde::de::DeserializeOwned>(
    path: &PathBuf,
) -> Result<T, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let result: T = serde_json::from_reader(reader)?;
    Ok(result)
}

pub fn save_model_json<T: serde::Serialize>(
    model: &T,
    path: &PathBuf,
) -> Result<(), std::io::Error> {
    serde_json::to_writer(&File::create(path)?, model)?;
    Ok(())
}

impl ApiService {
    // config priority:
    // args >> api_config.json >> anonymous api usage only
    pub fn new(config: PrintNannyConfig) -> Result<ApiService, ServiceError> {
        debug!("Initializing ApiService from config: {:?}", config);
        let paths = PrintNannyPath::new(&config.path);
        let reqwest = ReqwestConfig::from(&config.api);
        Ok(Self {
            reqwest,
            paths,
            config,
            device: None,
            user: None,
        })
    }

    pub fn api_config_save(&self, bearer_access_token: &str) -> Result<(), ServiceError> {
        let base_path = self.reqwest.base_path.to_string();
        let config = models::PrintNannyApiConfig {
            bearer_access_token: bearer_access_token.to_string(),
            base_path,
        };
        info!("Saving api_config to {:?}", &self.paths.api_config_json);
        save_model_json::<models::PrintNannyApiConfig>(&config, &self.paths.api_config_json)?;
        Ok(())
    }

    // auth APIs

    // fetch user associated with auth token
    pub async fn auth_user_retreive(&self) -> Result<models::User, ServiceError> {
        Ok(users_api::users_me_retrieve(&self.reqwest).await?)
    }

    pub async fn auth_email_create(
        &self,
        email: String,
    ) -> Result<models::DetailResponse, ServiceError> {
        let req = models::EmailAuthRequest { email };
        Ok(auth_api::auth_email_create(&self.reqwest, req).await?)
    }
    pub async fn auth_token_validate(
        &self,
        email: &str,
        token: &str,
    ) -> Result<models::TokenResponse, ServiceError> {
        let req = models::CallbackTokenAuthRequest {
            email: Some(email.to_string()),
            token: token.to_string(),
            mobile: None,
        };
        Ok(auth_api::auth_token_create(&self.reqwest, req).await?)
    }

    // device API
    pub async fn device_create(&self) -> Result<models::Device, ServiceError> {
        let hostname = sys_info::hostname()?;
        let req = models::DeviceRequest {
            hostname: Some(hostname),
            monitoring_active: Some(false),
            release_channel: None,
        };
        Ok(devices_api::devices_create(&self.reqwest, req).await?)
    }

    pub async fn device_retrieve_hostname(&self) -> Result<models::Device, ServiceError> {
        let hostname = sys_info::hostname()?;
        let res = devices_api::devices_retrieve_hostname(&self.reqwest, &hostname).await?;
        Ok(res)
    }

    pub async fn device_retrieve_or_create_hostname(&self) -> Result<models::Device, ServiceError> {
        let res = self.device_retrieve_hostname().await;
        match res {
            Ok(device) => Ok(device),
            // handle 404 / Not Found error by attempting to create device with hostname
            Err(e) => match &e {
                ServiceError::DevicesRetrieveHostnameError(ApiError::ResponseError(content)) => {
                    match content.status {
                        reqwest::StatusCode::NOT_FOUND => {
                            warn!("Failed retreive device with error={:?} - attempting to create device", e);
                            let res = self.device_create().await?;
                            info!("Success! Created device={:?}", res);
                            Ok(res)
                        }
                        _ => Err(e),
                    }
                }
                _ => Err(e),
            },
        }
    }

    pub async fn device_setup(&self) -> Result<models::Device, ServiceError> {
        // get or create device with matching hostname
        let device = self.device_retrieve_or_create_hostname().await?;
        info!("Success! Registered device: {:?}", device);
        // create SystemInfo
        let system_info = self.device_system_info_update_or_create(device.id).await?;
        info!("Success! Updated SystemInfo {:?}", system_info);

        // create JanusAuth
        self.device_janus_auth_update_or_create(device.id).await?;
        info!("Success! Updated JanusAuth");
        // create PublicKey
        let public_key = self.device_public_key_update_or_create(device.id).await?;
        info!("Success! Updated PublicKey: {:?}", public_key);

        // get user
        let user = self.auth_user_retreive().await?;
        // save License.toml with user/device info
        let mut config = self.config.clone();
        config.device = Some(device.clone());
        config.user = Some(user);
        config.system_info = Some(system_info);
        config.save();
        Ok(device)
    }

    async fn device_public_key_update_or_create(
        &self,
        device: i32,
    ) -> Result<models::PublicKey, ServiceError> {
        info!("Reading public key from {:?}", &self.paths.public_key);
        let pem = read_to_string(&self.paths.public_key)?;
        let cipher = models::CipherEnum::Ecdsa;
        let length = 256;
        info!("Calculating fingerprint for {:?}", &self.paths.public_key);
        let output = Command::new("openssl")
            .args([
                "sha3-256",
                "-c",
                &self.paths.public_key.as_os_str().to_str().unwrap(),
            ])
            .output()
            .expect("Failed to get fingerprint");
        if output.status.success() {
            let fingerprint: String = String::from_utf8(output.stdout)?;
            info!("Calculated fingerprint {:?}", fingerprint);

            let req = models::PublicKeyRequest {
                fingerprint,
                pem,
                cipher,
                length,
                device,
            };

            let res = devices_api::public_key_update_or_create(&self.reqwest, device, req).await?;
            Ok(res)
        } else {
            error!("Error calculating fingerprint {:?}", output);

            error!("Error calculating fingerprint {:?}", output.stderr);
            Err(ServiceError::FingerprintError {
                path: self.paths.public_key.clone(),
                stderr: std::str::from_utf8(&output.stderr)?.to_string(),
            })
        }
    }

    async fn device_janus_auth_update_or_create(
        &self,
        device: i32,
    ) -> Result<models::JanusAuth, ServiceError> {
        info!(
            "Reading janus_admin_secret from {:?}",
            &self.paths.janus_admin_secret
        );
        let janus_admin_secret = read_to_string(&self.paths.janus_admin_secret)?;
        info!("Reading janus_token from {:?}", &self.paths.janus_token);
        let janus_token = read_to_string(&self.paths.janus_token)?;
        let req = models::JanusAuthRequest {
            janus_token,
            janus_admin_secret,
            device,
        };
        let res = devices_api::janus_auth_update_or_create(&self.reqwest, device, req).await?;
        Ok(res)
    }

    async fn device_system_info_update_or_create(
        &self,
        device: i32,
    ) -> Result<models::SystemInfo, ServiceError> {
        let machine_id: String = read_to_string("/etc/machine-id")?;

        // hacky parsing of rpi-specific /proc/cpuinfo
        let rpi_cpuinfo = RpiCpuInfo::new();
        let hardware = rpi_cpuinfo.hardware.unwrap_or("Unknown".to_string());
        let model = rpi_cpuinfo.model.unwrap_or("Unknown".to_string());
        let serial = rpi_cpuinfo.serial.unwrap_or("Unknown".to_string());
        let revision = rpi_cpuinfo.revision.unwrap_or("Unknown".to_string());

        let cpuinfo = procfs::CpuInfo::new()?;
        let cores: i32 = cpuinfo.num_cores().try_into().unwrap();

        let meminfo = procfs::Meminfo::new()?;
        let ram = meminfo.mem_total.try_into().unwrap();

        let image_version = read_to_string("/boot/image_version.txt")
            .unwrap_or("Failed to parse /boot/image_version.txt".to_string());

        let request = models::SystemInfoRequest {
            machine_id,
            hardware,
            serial,
            revision,
            model,
            cores,
            ram,
            image_version,
            device,
        };
        let res = devices_api::system_info_update_or_create(&self.reqwest, device, request).await?;
        Ok(res)
    }

    // read <models::<T>>.json from disk cache @ /var/run/printnanny
    // hydrate cache if not found using fallback fn f (must return a Future)
    pub async fn load_model<T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug>(
        &self,
        path: &PathBuf,
        f: impl Future<Output = Result<T, ServiceError>>,
    ) -> Result<T, ServiceError> {
        let m = read_model_json::<T>(path);
        match m {
            Ok(v) => Ok(v),
            Err(_e) => {
                warn!(
                    "Failed to read {:?} - falling back to load remote model",
                    path
                );
                let res = f.await;
                match res {
                    Ok(v) => {
                        save_model_json::<T>(&v, path)?;
                        info!("Saved model {:?} to {:?}", &v, path);
                        Ok(v)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    // read device.json from disk cache @ /var/run/printnanny
    // hydrate cache if device.json not found
    // pub async fn load_device_json(&self) -> Result<models::Device, ServiceError> {
    //     let m = read_model_json::<models::Device>(&self.paths.device_json);
    //     match m {
    //         Ok(device) => Ok(device),
    //         Err(_e) => {
    //             warn!("Failed to read {:?} - attempting to load device.json from remote", &self.paths.device_json);
    //             let res = self.device_retrieve_hostname().await;
    //             match res {
    //                 Ok(device) => {
    //                     save_model_json::<models::Device>(&device, &self.paths.device_json)?;
    //                     info!("Saved model {:?} to {:?}", &device, &self.paths.device_json);
    //                     Ok(device)
    //                 }
    //                 Err(e) => Err(e)
    //             }
    //         }
    //     }
    // }

    pub async fn task_status_create(
        &self,
        task_id: i32,
        device_id: i32,
        status: models::TaskStatusType,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<models::TaskStatus, ServiceError> {
        let request = models::TaskStatusRequest {
            detail,
            wiki_url,
            task: task_id,
            status,
        };
        info!("Submitting TaskStatusRequest={:?}", request);
        let res =
            devices_api::devices_tasks_status_create(&self.reqwest, device_id, task_id, request)
                .await?;
        Ok(res)
    }

    // pub async fn task_create(
    //     &self,
    //     task_type: models::TaskType,
    //     status: Option<models::TaskStatusType>,
    //     detail: Option<String>,
    //     wiki_url: Option<String>
    // ) -> Result<models::Task, ServiceError> {
    //     match &self.device {
    //         Some(device) => {
    //             let request = models::TaskRequest{
    //                 active: Some(true),
    //                 task_type,
    //                 device: device.id
    //             };
    //             let task = devices_api::devices_tasks_create(&self.reqwest, device.id, request).await?;
    //             info!("Success: created task={:?}", task);
    //             match status {
    //                 Some(s) => {
    //                     let res  = self.task_status_create(task.id, device.id, s, wiki_url, detail ).await?;
    //                     info!("Success: created task status={:?}", res);
    //                     Ok(task)
    //                 },
    //                 None => Ok(task)
    //             }
    //         },
    //         None => Err(ServiceError::SignupIncomplete{ cache: self.paths.device_json.clone() })
    //     }
    // }
    pub fn to_string_pretty<T: serde::Serialize>(
        &self,
        item: T,
    ) -> serde_json::error::Result<String> {
        serde_json::to_string_pretty::<T>(&item)
    }
}

use log::{debug, error, info, warn};
use std::convert::TryInto;
use std::fs::{read_to_string, File};
use std::future::Future;
use std::io::BufReader;
use std::path::PathBuf;

use printnanny_api_client::apis::auth_api;
use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::janus_api;
use printnanny_api_client::apis::users_api;
use printnanny_api_client::apis::Error as ApiError;
use printnanny_api_client::models;
use thiserror::Error;

use crate::config::PrintNannyConfig;
use crate::cpuinfo::RpiCpuInfo;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    AuthTokenCreateError(#[from] ApiError<auth_api::AuthTokenCreateError>),
    #[error(transparent)]
    AuthEmailCreateError(#[from] ApiError<auth_api::AuthEmailCreateError>),

    #[error(transparent)]
    CloudiotDeviceUpdateOrCreateError(
        #[from] ApiError<devices_api::CloudiotDeviceUpdateOrCreateError>,
    ),

    #[error(transparent)]
    DevicesCreateError(#[from] ApiError<devices_api::DevicesCreateError>),

    #[error(transparent)]
    DevicesRetrieveError(#[from] ApiError<devices_api::DevicesRetrieveError>),

    #[error(transparent)]
    DevicesPartialUpdateError(#[from] ApiError<devices_api::DevicesPartialUpdateError>),

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
    ProcError(#[from] procfs::ProcError),

    #[error(transparent)]
    FigmentError(#[from] figment::Error),

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
        let reqwest = ReqwestConfig::from(&config.api);
        Ok(Self {
            reqwest,
            config,
            device: None,
            user: None,
        })
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
            setup_complete: Some(false),
        };
        Ok(devices_api::devices_create(&self.reqwest, req).await?)
    }

    pub async fn device_retrieve_hostname(&self) -> Result<models::Device, ServiceError> {
        let hostname = sys_info::hostname()?;
        let res = devices_api::devices_retrieve_hostname(&self.reqwest, &hostname).await?;
        Ok(res)
    }

    pub async fn cloudiot_device_update_or_create(
        &self,
        device: i32,
        public_key: i32,
    ) -> Result<models::CloudiotDevice, ServiceError> {
        let req = models::CloudiotDeviceRequest { public_key };
        let res = devices_api::cloudiot_device_update_or_create(&self.reqwest, device, req).await?;
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

    pub async fn device_patch(
        &self,
        device_id: i32,
        request: models::PatchedDeviceRequest,
    ) -> Result<models::Device, ServiceError> {
        let res =
            devices_api::devices_partial_update(&self.reqwest, device_id, Some(request)).await?;
        info!(
            "Success! Patched device_id={} with response={:?}",
            device_id, res
        );
        Ok(res)
    }

    pub async fn device_setup(&self) -> Result<PrintNannyConfig, ServiceError> {
        // get or create device with matching hostname
        let device = self.device_retrieve_or_create_hostname().await?;
        info!("Success! Registered device: {:?}", device);
        // create SystemInfo
        let system_info = self.device_system_info_update_or_create(device.id).await?;
        info!("Success! Updated SystemInfo {:?}", system_info);

        // get or create cloud JanusAuth
        let janus_cloud = self
            .janus_stream_get_or_create(device.id, models::JanusConfigType::Cloud)
            .await?;
        info!("Success! Retreived JanusStream {:?}", janus_cloud);
        // create PublicKey
        let public_key = self.device_public_key_update_or_create(device.id).await?;
        info!("Success! Updated PublicKey: {:?}", public_key);

        let _cloudiot_device = self
            .cloudiot_device_update_or_create(device.id, public_key.id)
            .await?;
        // refresh user
        let user = self.auth_user_retreive().await?;
        // save License.toml with user/device info
        let mut config = self.config.clone();
        let patched = models::PatchedDeviceRequest {
            setup_complete: Some(true),
            monitoring_active: None,
            release_channel: None,
            hostname: None,
        };
        let device = self.device_patch(device.id, patched).await?;
        config.device = Some(device.clone());
        config.user = Some(user);
        config.janus_cloud = Some(janus_cloud);
        config.save()?;
        Ok(config)
    }

    async fn device_public_key_update_or_create(
        &self,
        device: i32,
    ) -> Result<models::PublicKey, ServiceError> {
        info!("Reading public key from {:?}", &self.config.mqtt.public_key);
        let pem = read_to_string(&self.config.mqtt.public_key)?;
        let req = models::PublicKeyRequest {
            fingerprint: self.config.mqtt.fingerprint.clone(),
            pem,
            device,
            cipher: self.config.mqtt.cipher.clone(),
            length: self.config.mqtt.length.clone(),
        };
        let res = devices_api::public_key_update_or_create(&self.reqwest, device, req).await?;
        Ok(res)
    }

    async fn janus_stream_get_or_create(
        &self,
        device: i32,
        config_type: models::JanusConfigType,
    ) -> Result<models::JanusStream, ServiceError> {
        // None fields will be generated server-side
        let req = models::JanusStreamRequest {
            config_type: Some(config_type),
            active: Some(false),
            secret: None,
            pin: None,
            info: None,
        };
        let res =
            janus_api::devices_janus_stream_get_or_create(&self.reqwest, device, Some(req)).await?;
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

    pub async fn task_status_create(
        &self,
        task_id: i32,
        device_id: i32,
        status: models::TaskStatusType,
        detail: Option<String>,
        wiki_url: Option<String>,
    ) -> Result<models::TaskStatus, ServiceError> {
        let request = models::TaskStatusRequest { status };
        info!("Submitting TaskStatusRequest={:?}", request);
        let res =
            devices_api::devices_tasks_status_create(&self.reqwest, device_id, task_id, request)
                .await?;
        Ok(res)
    }
    pub fn to_string_pretty<T: serde::Serialize>(
        &self,
        item: T,
    ) -> serde_json::error::Result<String> {
        serde_json::to_string_pretty::<T>(&item)
    }
}

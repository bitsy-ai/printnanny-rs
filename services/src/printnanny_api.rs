use log::{debug, info, warn};
use std::collections::HashMap;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::path::Path;

use printnanny_api_client::apis::accounts_api;
use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::models;

use super::config::PrintNannyConfig;
use super::error::{PrintNannyConfigError, ServiceError};
use super::file::open;
use super::metadata;
use super::octoprint::OctoPrintHelper;

#[derive(Debug, Clone)]
pub struct ApiService {
    pub reqwest: ReqwestConfig,
    pub config: PrintNannyConfig,
    pub pi: Option<models::Pi>,
    pub user: Option<models::User>,
}

pub fn read_model_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, std::io::Error> {
    let file = open(path)?;
    let reader = BufReader::new(file);
    let result: T = serde_json::from_reader(reader)?;
    Ok(result)
}

pub fn save_model_json<T: serde::Serialize>(model: &T, path: &Path) -> Result<(), std::io::Error> {
    serde_json::to_writer(&File::create(path)?, model)?;
    Ok(())
}

impl ApiService {
    // config priority:
    // args >> api_config.json >> anonymous api usage only
    pub fn new(config: PrintNannyConfig) -> Result<ApiService, ServiceError> {
        debug!("Initializing ApiService from config: {:?}", config);

        let reqwest = ReqwestConfig {
            base_path: config.cloud.api.base_path.to_string(),
            bearer_access_token: config.cloud.api.bearer_access_token.clone(),
            ..ReqwestConfig::default()
        };
        Ok(Self {
            reqwest,
            config,
            pi: None,
            user: None,
        })
    }
    // // alert settings API
    // pub async fn alert_settings_get_or_create(
    //     &self,
    // ) -> Result<models::AlertSettings, ServiceError> {
    //     let res = alert_settings_api::alert_settings_get_or_create_retrieve(&self.reqwest).await?;
    //     Ok(res)
    // }
    pub async fn auth_user_retreive(&self) -> Result<models::User, ServiceError> {
        Ok(accounts_api::accounts_user_retrieve(&self.reqwest).await?)
    }

    pub async fn auth_email_create(
        &self,
        email: String,
    ) -> Result<models::EmailAuth, ServiceError> {
        let req = models::EmailAuthRequest { email };
        Ok(accounts_api::accounts2fa_auth_email_create(&self.reqwest, req).await?)
    }
    pub async fn auth_token_validate(
        &self,
        email: &str,
        token: &str,
    ) -> Result<models::CallbackTokenAuth, ServiceError> {
        let req = models::CallbackTokenAuthRequest {
            email: Some(email.to_string()),
            token: token.to_string(),
            mobile: None,
        };
        Ok(accounts_api::accounts2fa_auth_token_create(&self.reqwest, req).await?)
    }

    // syncs Raspberry Pi data with PrintNanny Cloud
    // performs any necessary one-time setup tasks
    pub async fn sync(&mut self) -> Result<(), ServiceError> {
        // verify pi is authenticated

        match &self.config.cloud.pi {
            Some(pi) => {
                info!("Pi is already registered: {:?}", self.pi);
                // always update SystemInfo
                info!("Calling device_system_info_update_or_create()");
                let system_info = self.system_info_update_or_create(pi.id).await?;
                info!("Success! Updated SystemInfo {:?}", system_info);

                // detect edition from os-release
                // let os_release = self.config.paths.load_os_release()?;
                match &pi.octoprint_server {
                    Some(octoprint_server) => {
                        self.octoprint_server_update(octoprint_server).await?;
                    }
                    None => (),
                }

                let pi = self.pi_retrieve(pi.id).await?;
                self.config.cloud.pi = Some(pi);
                Ok(())
            }
            None => {
                warn!("Pi is not registered, attempting to register");

                let pi = 

            }
        };

        self.config.try_save()?

    }

    pub async fn pi_retrieve(&self, pi_id: i32) -> Result<models::Pi, ServiceError> {
        let res = devices_api::pis_retrieve(&self.reqwest, pi_id).await?;
        Ok(res)
    }

    pub async fn pi_partial_update(
        &self,
        pi_id: i32,
        req: models::PatchedPiRequest,
    ) -> Result<models::Pi, ServiceError> {
        let res = devices_api::pis_partial_update(&self.reqwest, pi_id, Some(req)).await?;
        Ok(res)
    }

    async fn system_info_update_or_create(
        &self,
        pi: i32,
    ) -> Result<models::SystemInfo, ServiceError> {
        let system_info = metadata::system_info()?;
        let os_release_json: HashMap<String, serde_json::Value> =
            serde_json::from_str(&serde_json::to_string(&system_info.os_release)?)?;

        let request = models::SystemInfoRequest {
            pi,
            os_build_id: system_info.os_release.build_id,
            os_variant_id: system_info.os_release.variant_id,
            os_version_id: system_info.os_release.version_id,
            os_release_json: Some(os_release_json),

            machine_id: system_info.machine_id,
            serial: system_info.serial,
            revision: system_info.revision,
            model: system_info.model,
            cores: system_info.cores,
            ram: system_info.ram,
            bootfs_size: system_info.bootfs_size,
            bootfs_used: system_info.bootfs_used,
            datafs_size: system_info.datafs_size,
            datafs_used: system_info.datafs_used,
            rootfs_size: system_info.rootfs_size,
            rootfs_used: system_info.rootfs_used,
            uptime: system_info.uptime,
        };
        info!("device_system_info_update_or_create request {:?}", request);
        let res = devices_api::system_info_update_or_create(&self.reqwest, pi, request).await?;
        Ok(res)
    }

    pub async fn octoprint_server_update(
        &self,
        octoprint_server: &models::OctoPrintServer,
    ) -> Result<models::OctoPrintServer, ServiceError> {
        let helper = OctoPrintHelper::new(octoprint_server.clone());
        let pip_version = helper.pip_version()?;
        let python_version = helper.python_version()?;
        let pip_packages = helper.pip_packages()?;
        let octoprint_version = helper.octoprint_version(&pip_packages)?.into();
        let printnanny_plugin_version = helper.printnanny_plugin_version(&pip_packages)?;
        let req = models::PatchedOctoPrintServerRequest {
            octoprint_version,
            pip_version,
            printnanny_plugin_version,
            python_version,
            pi: Some(helper.octoprint_server.pi),
            ..models::PatchedOctoPrintServerRequest::new()
        };
        debug!(
            "Sending request {:?} to octoprint_server_update_or_create",
            req
        );
        let res =
            octoprint_api::octoprint_partial_update(&self.reqwest, octoprint_server.id, Some(req))
                .await?;
        Ok(res)
    }

    // read <models::<T>>.json from disk cache @ /var/run/printnanny
    // hydrate cache if not found using fallback fn f (must return a Future)
    pub async fn load_model<T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug>(
        &self,
        path: &Path,
        f: impl Future<Output = Result<T, PrintNannyConfigError>>,
    ) -> Result<T, PrintNannyConfigError> {
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
                        match save_model_json::<T>(&v, path) {
                            Ok(()) => Ok(()),
                            Err(error) => Err(PrintNannyConfigError::WriteIOError {
                                path: path.to_path_buf(),
                                error,
                            }),
                        }?;
                        info!("Saved model {:?} to {:?}", &v, path);
                        Ok(v)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}

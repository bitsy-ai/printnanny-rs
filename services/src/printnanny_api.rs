use log::{debug, error, info, warn};
use printnanny_settings::vcs::VersionControlledSettings;
use std::collections::HashMap;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde;
use serde_json;
use tempfile::NamedTempFile;

// settings modules
use printnanny_settings::cloud::PrintNannyCloudData;
use printnanny_settings::error::PrintNannySettingsError;
use printnanny_settings::printnanny::PrintNannySettings;

use printnanny_settings::sys_info;

use printnanny_api_client::apis::accounts_api;
use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::apis::crash_reports_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::models;

use crate::cpuinfo::RpiCpuInfo;
use crate::crash_report::write_crash_report_zip;
use crate::error::ServiceError;
use crate::file::open;
use crate::metadata;
use crate::os_release::OsRelease;

#[derive(Debug, Clone)]
pub struct ApiService {
    pub settings: PrintNannySettings,
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
    pub fn new() -> Result<ApiService, ServiceError> {
        let settings = PrintNannySettings::new()?;
        debug!("Initializing ApiService from settings: {:?}", settings);

        Ok(Self {
            settings,
            pi: None,
            user: None,
        })
    }

    fn reqwest_config(&self) -> ReqwestConfig {
        ReqwestConfig {
            base_path: self.settings.cloud.api_base_path.clone(),
            bearer_access_token: self.settings.cloud.api_bearer_access_token.clone(),
            ..ReqwestConfig::default()
        }
    }

    pub async fn connect_cloud_account(
        mut self,
        api_base_path: String,
        api_bearer_access_token: String,
    ) -> Result<Self, ServiceError> {
        let previous = self.settings.clone();
        self.settings.cloud.api_base_path = api_base_path;
        self.settings.cloud.api_bearer_access_token = Some(api_bearer_access_token);
        if previous != self.settings {
            warn!("Change in PrintNannySettings detected, commiting changes");
            let content = self.settings.to_toml_string()?;
            self.settings
                .save_and_commit(
                    &content,
                    Some("Updated PrintNanny Cloud API auth".to_string()),
                )
                .await?;
        }

        let cloud_state_file = self.settings.paths.cloud();

        // sync data models
        self.sync().await?;
        let mut state = PrintNannyCloudData::load(&cloud_state_file)?;
        let pi_id = state.pi.unwrap().id;

        // download license
        if !self.settings.paths.license_zip().exists() {
            // download credential and device identity bundled in license.zip
            self.pi_download_license(pi_id, false).await?;
        }
        // mark setup complete
        let req = models::PatchedPiRequest {
            setup_finished: Some(true),
            // None values are skipped by serde serializer
            sbc: None,
            hostname: None,
            favorite: None,
        };

        self.pi_partial_update(pi_id, req).await?;
        let pi = self.pi_retrieve(pi_id).await?;
        state.pi = Some(pi);
        state.save(&cloud_state_file)?;
        Ok(self)
    }

    pub async fn crash_report_create(
        &self,
        description: Option<&str>,
        email: Option<&str>,
        browser_version: Option<&str>,
        browser_logs: Option<PathBuf>,
        status: Option<models::CrashReportStatusEnum>,
        posthog_session: Option<&str>,
    ) -> Result<models::CrashReport, ServiceError> {
        let file = NamedTempFile::new()?;
        let (file, filename) = &file.keep()?;

        write_crash_report_zip(file)?;
        warn!("Wrote crash report logs to {}", filename.display());

        let serial = match RpiCpuInfo::new() {
            Ok(rpi_cpuinfo) => rpi_cpuinfo.serial,
            Err(e) => {
                error!("Failed to read RpiCpuInfo with error={}", e);
                None
            }
        };

        let os_release = OsRelease::new()?;

        let pi = self.pi.as_ref().map(|pi| pi.id);

        let user = self.user.as_ref().map(|user| user.id);

        let result = crash_reports_api::crash_reports_create(
            &self.reqwest_config(),
            description,
            email,
            Some(&os_release.version),
            Some(filename.to_path_buf()),
            browser_version,
            browser_logs,
            serial.as_deref(),
            posthog_session,
            status,
            None,
            user,
            pi,
        )
        .await?;
        Ok(result)
    }

    pub async fn crash_report_update(&self, id: &str) -> Result<models::CrashReport, ServiceError> {
        let os_release = OsRelease::new()?;
        let file = NamedTempFile::new()?;
        let (file, filename) = &file.keep()?;

        write_crash_report_zip(file)?;
        warn!("Wrote crash report logs to {}", filename.display());

        let serial = match RpiCpuInfo::new() {
            Ok(rpi_cpuinfo) => rpi_cpuinfo.serial,
            Err(e) => {
                error!("Failed to read RpiCpuInfo with error={}", e);
                None
            }
        };

        let pi = self.pi.as_ref().map(|pi| pi.id);

        let user = self.user.as_ref().map(|user| user.id);

        let result = crash_reports_api::crash_reports_partial_update(
            &self.reqwest_config(),
            id,
            None,
            None,
            Some(&os_release.version),
            Some(filename.to_path_buf()),
            None,
            None,
            serial.as_deref(),
            None,
            None,
            None,
            user,
            pi,
        )
        .await?;

        Ok(result)
    }

    pub async fn auth_user_retreive(&self) -> Result<models::User, ServiceError> {
        Ok(accounts_api::accounts_user_retrieve(&self.reqwest_config()).await?)
    }

    pub async fn auth_email_create(
        &self,
        email: String,
    ) -> Result<models::EmailAuth, ServiceError> {
        let req = models::EmailAuthRequest { email };
        Ok(accounts_api::accounts2fa_auth_email_create(&self.reqwest_config(), req).await?)
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
        Ok(accounts_api::accounts2fa_auth_token_create(&self.reqwest_config(), req).await?)
    }

    async fn sync_pi_models(&self, pi: &models::Pi) -> Result<models::Pi, ServiceError> {
        info!(
            "Synchronizing models for Pi with id={}: device_system_info_update_or_create()",
            pi.id
        );
        let system_info = self.system_info_update_or_create(pi.id).await?;
        info!("Success! Updated SystemInfo model: {:?}", system_info);
        match &pi.octoprint_server {
            Some(octoprint_server) => {
                let octoprint_server = self.octoprint_server_update(octoprint_server).await?;
                info!(
                    "Success! Updated OctoPrintServer model: {:?}",
                    octoprint_server
                );
            }
            None => (),
        }

        let pi = self.pi_retrieve(pi.id).await?;
        Ok(pi)
    }

    async fn init_pi_model(&self) -> Result<models::Pi, ServiceError> {
        warn!("Pi is not registered, attempting to register");
        // TODO detect board, but for now only Raspberry Pi 4 is supported so
        let _sbc = Some(models::SbcEnum::Rpi4);
        let hostname = sys_info::hostname().unwrap_or_else(|_| "printnanny".to_string());

        let favorite = Some(true);
        let setup_finished = Some(true);

        let req = models::PiRequest {
            sbc: Some(models::SbcEnum::Rpi4),
            hostname: Some(hostname),
            favorite,
            setup_finished,
        };
        let pi = devices_api::pi_update_or_create(&self.reqwest_config(), Some(req)).await?;
        info!("Success! Registered Pi: {:#?}", pi);
        let pi = self.sync_pi_models(&pi).await?;
        Ok(pi)
    }

    // syncs Raspberry Pi data with PrintNanny Cloud
    // performs any necessary one-time setup tasks
    pub async fn sync(&mut self) -> Result<(), ServiceError> {
        // ensure directory structure exists
        self.settings.paths.try_init_all()?;

        // is there existing state to load?
        let cloud_state_file = self.settings.paths.cloud();
        match cloud_state_file.exists() {
            true =>
            // verify pi is authenticated
            {
                let mut state = match PrintNannyCloudData::load(&cloud_state_file) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Error loading PrintNannyCloudData: {}", e);
                        PrintNannyCloudData::default()
                    }
                };
                match &state.pi {
                    Some(pi) => {
                        info!(
                            "Pi is already registered, updating related models for {:?}",
                            pi
                        );

                        let pi = self.sync_pi_models(pi).await?;
                        state.pi = Some(pi);
                    }
                    None => {
                        let pi = self.init_pi_model().await?;
                        state.pi = Some(pi);
                    }
                };

                state.save(&cloud_state_file)?;
            }
            false => {
                let mut state = PrintNannyCloudData::default();
                let pi = self.init_pi_model().await?;
                state.pi = Some(pi);
                state.save(&cloud_state_file)?;
            }
        };

        Ok(())
    }

    pub async fn pi_retrieve(&self, pi_id: i32) -> Result<models::Pi, ServiceError> {
        let res = devices_api::pis_retrieve(&self.reqwest_config(), pi_id).await?;
        Ok(res)
    }

    pub async fn pi_partial_update(
        &self,
        pi_id: i32,
        req: models::PatchedPiRequest,
    ) -> Result<models::Pi, ServiceError> {
        let res = devices_api::pis_partial_update(&self.reqwest_config(), pi_id, Some(req)).await?;
        Ok(res)
    }

    pub async fn pi_download_license(&self, pi_id: i32, backup: bool) -> Result<(), ServiceError> {
        let res = devices_api::pis_license_zip_retrieve(&self.reqwest_config(), pi_id).await?;
        self.settings.paths.write_license_zip(res, backup)?;
        self.settings.paths.unpack_license(backup)?;
        Ok(())
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
            os_build_id: Some(system_info.os_release.build_id),
            os_version_id: Some(system_info.os_release.version_id),
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
        let res =
            devices_api::system_info_update_or_create(&self.reqwest_config(), pi, request).await?;
        Ok(res)
    }

    pub async fn octoprint_server_update(
        &self,
        octoprint_server: &models::OctoPrintServer,
    ) -> Result<models::OctoPrintServer, ServiceError> {
        let helper = &self.settings.octoprint;
        let pip_version = helper.pip_version()?;
        let python_version = helper.python_version()?;
        let pip_packages = helper.pip_packages()?;
        let octoprint_version = helper.octoprint_version(&pip_packages);
        let printnanny_plugin_version = helper.printnanny_plugin_version(&pip_packages);
        let req = models::PatchedOctoPrintServerRequest {
            octoprint_version,
            pip_version,
            printnanny_plugin_version,
            python_version,
            pi: Some(octoprint_server.pi),
            ..models::PatchedOctoPrintServerRequest::new()
        };
        debug!(
            "Sending request {:?} to octoprint_server_update_or_create",
            req
        );
        let res = octoprint_api::octoprint_partial_update(
            &self.reqwest_config(),
            octoprint_server.id,
            Some(req),
        )
        .await?;
        Ok(res)
    }

    // read <models::<T>>.json from disk cache @ /var/run/printnanny
    // hydrate cache if not found using fallback fn f (must return a Future)
    pub async fn load_model<T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug>(
        &self,
        path: &Path,
        f: impl Future<Output = Result<T, PrintNannySettingsError>>,
    ) -> Result<T, PrintNannySettingsError> {
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
                            Err(error) => Err(PrintNannySettingsError::WriteIOError {
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

use log::{debug, error, info, warn};
use printnanny_settings::vcs::VersionControlledSettings;
use std::collections::HashMap;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde;
use serde_json;
use tokio::fs;

// edge db
// use printnanny_edge_db::cloud::PrintNannyCloudApiConfig;

// settings modules
use printnanny_settings::error::PrintNannySettingsError;
use printnanny_settings::printnanny::{PrintNannyApiConfig, PrintNannySettings};
use printnanny_settings::sys_info;

use printnanny_api_client::apis::accounts_api;
use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::apis::crash_reports_api;
use printnanny_api_client::apis::devices_api;
use printnanny_api_client::apis::octoprint_api;
use printnanny_api_client::apis::videos_api;
use printnanny_api_client::models;

use crate::cpuinfo::RpiCpuInfo;
use crate::crash_report::write_crash_report_zip;
use crate::error::{ServiceError, VideoRecordingUpdateOrCreateError};
use crate::file::open;
use crate::metadata;
use crate::os_release::OsRelease;

#[derive(Debug, Clone)]
pub struct ApiService {
    pub sqlite_connection: String,
    pub api_config: PrintNannyApiConfig,
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
    pub fn new(api_config: PrintNannyApiConfig, sqlite_connection: String) -> Self {
        Self {
            api_config,
            sqlite_connection,
            pi: None,
            user: None,
        }
    }

    fn reqwest_config(&self) -> ReqwestConfig {
        ReqwestConfig {
            base_path: self.api_config.api_base_path.clone(),
            bearer_access_token: self.api_config.api_bearer_access_token.clone(),
            ..ReqwestConfig::default()
        }
    }

    pub async fn connect_cloud_account(
        mut self,
        api_base_path: String,
        api_bearer_access_token: String,
    ) -> Result<Self, ServiceError> {
        let mut settings = PrintNannySettings::new().await?;

        info!("Updated printnanny_cloud_api_config sqlite record");
        let api_bearer_access_token = Some(api_bearer_access_token);
        if self.api_config.api_bearer_access_token != api_bearer_access_token {
            self.api_config.api_base_path = api_base_path.clone();
            self.api_config.api_bearer_access_token = api_bearer_access_token.clone();
            warn!("connect_cloud_account saving new api_bearer_access_token");
            settings.cloud.api_base_path = api_base_path;
            settings.cloud.api_bearer_access_token = api_bearer_access_token;
            let content = settings.to_toml_string()?;
            settings
                .save_and_commit(
                    &content,
                    Some("Updated PrintNanny Cloud API auth".to_string()),
                )
                .await?;
        }

        // sync data models
        self.sync().await?;
        let pi_id = printnanny_edge_db::cloud::Pi::get_id(&self.sqlite_connection)?;

        // download license
        if settings.paths.license_zip().exists() {
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
        crash_report_paths: Vec<PathBuf>,
    ) -> Result<models::CrashReport, ServiceError> {
        let file = tempfile::Builder::new()
            .prefix("crash-report")
            .suffix(".zip")
            .rand_bytes(6)
            .tempfile()?;
        let (file, filename) = &file.keep()?;

        write_crash_report_zip(file, crash_report_paths).await?;
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
            pi,
        )
        .await?;
        warn!("Finished uploading {}, removing file", filename.display());
        fs::remove_file(filename).await?;
        Ok(result)
    }

    pub async fn crash_report_update(
        &self,
        id: &str,
        crash_report_paths: Vec<PathBuf>,
    ) -> Result<models::CrashReport, ServiceError> {
        let os_release = OsRelease::new()?;
        let file = tempfile::Builder::new()
            .prefix("crash-report")
            .suffix(".zip")
            .rand_bytes(6)
            .tempfile()?;
        let (file, filename) = &file.keep()?;

        write_crash_report_zip(file, crash_report_paths).await?;
        warn!("Wrote crash report logs to {}", filename.display());

        let serial = match RpiCpuInfo::new() {
            Ok(rpi_cpuinfo) => rpi_cpuinfo.serial,
            Err(e) => {
                error!("Failed to read RpiCpuInfo with error={}", e);
                None
            }
        };

        let pi = self.pi.as_ref().map(|pi| pi.id);

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

    async fn sync_pi_models(
        &self,
        edge_pi: printnanny_edge_db::cloud::Pi,
    ) -> Result<models::Pi, ServiceError> {
        info!(
            "Synchronizing models for Pi with id={}: system_info_update_or_create()",
            edge_pi.id
        );
        let system_info = self.system_info_update_or_create(edge_pi.id).await?;
        info!("Success! Updated SystemInfo model: {:?}", system_info);
        match &edge_pi.octoprint_server_id {
            Some(octoprint_server_id) => {
                let octoprint_server = self
                    .octoprint_server_update(octoprint_server_id, &edge_pi.id)
                    .await?;
                info!(
                    "Success! Updated OctoPrintServer model: {:?}",
                    octoprint_server
                );
            }
            None => (),
        }

        let pi = self.pi_retrieve(edge_pi.id).await?;
        let pi_id = pi.id;
        let changeset: printnanny_edge_db::cloud::UpdatePi = pi.clone().into();
        printnanny_edge_db::cloud::Pi::update(&self.sqlite_connection, pi_id, changeset)?;
        Ok(pi)
    }

    async fn init_pi_model(&self) -> Result<models::Pi, ServiceError> {
        warn!("Pi is not registered, attempting to register");
        // TODO detect board, but for now only Raspberry Pi 4 is supported so
        let _sbc = Some(models::SbcEnum::Rpi4);
        let hostname = sys_info::hostname().unwrap_or_else(|_| "printnanny".to_string());

        let favorite = true;
        let setup_finished = true;

        let req = models::PiRequest {
            sbc: models::SbcEnum::Rpi4,
            hostname,
            favorite,
            setup_finished,
        };
        let pi = devices_api::pi_update_or_create(&self.reqwest_config(), req).await?;
        info!("Success! Registered Pi: {:#?}", &pi);
        printnanny_edge_db::cloud::Pi::insert(&self.sqlite_connection, pi.clone().into())?;
        let pi = self.sync_pi_models(pi.into()).await?;
        Ok(pi)
    }

    // syncs Raspberry Pi data with PrintNanny Cloud
    // performs any necessary one-time setup tasks
    pub async fn sync(&self) -> Result<printnanny_api_client::models::Pi, ServiceError> {
        match printnanny_edge_db::cloud::Pi::get(&self.sqlite_connection) {
            Ok(pi_sqlite) => self.sync_pi_models(pi_sqlite).await,
            Err(e) => match e {
                // if edge Pi model isn't found, initialize
                printnanny_edge_db::diesel::result::Error::NotFound => self.init_pi_model().await,
                // re-raise all other errors
                _ => Err(ServiceError::SqliteDBError(e)),
            },
        }
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
        let settings = PrintNannySettings::new().await?;
        let res = devices_api::pis_license_zip_retrieve(&self.reqwest_config(), pi_id).await?;
        settings.paths.write_license_zip(res, backup).await?;
        tokio::task::spawn_blocking(move || settings.paths.unpack_license(backup)).await??;
        // settings.paths.unpack_license(backup).await?;
        Ok(())
    }

    async fn system_info_update_or_create(
        &self,
        pi: i32,
    ) -> Result<models::SystemInfo, ServiceError> {
        let system_info = tokio::task::spawn_blocking(metadata::system_info).await??;
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
        octoprint_server_id: &i32,
        pi_id: &i32,
    ) -> Result<models::OctoPrintServer, ServiceError> {
        let settings = PrintNannySettings::new().await?;
        let helper = settings.to_octoprint_settings();
        let python_version = helper.python_version();
        let pip_version = helper.pip_version();
        let pip_packages = helper.pip_packages().await?;
        let octoprint_version = helper.octoprint_version(&pip_packages);
        let printnanny_plugin_version = helper.printnanny_plugin_version(&pip_packages);
        let req = models::PatchedOctoPrintServerRequest {
            octoprint_version,
            printnanny_plugin_version,
            pip_version: pip_version.await?,
            python_version: python_version.await?,
            pi: Some(*pi_id),
            ..models::PatchedOctoPrintServerRequest::new()
        };
        debug!(
            "Sending request {:?} to octoprint_server_update_or_create",
            req
        );
        let res = octoprint_api::octoprint_partial_update(
            &self.reqwest_config(),
            *octoprint_server_id,
            Some(req),
        )
        .await?;
        Ok(res)
    }

    pub async fn video_recordings_partial_update(
        &self,
        obj: &printnanny_edge_db::video_recording::VideoRecording,
    ) -> Result<models::VideoRecording, VideoRecordingUpdateOrCreateError> {
        let id = obj.id.as_str();
        let recording_start = obj.recording_start.map(|v| v.to_rfc3339());
        let recording_end = obj.recording_end.map(|v| v.to_rfc3339());
        let cloud_sync_start = obj.cloud_sync_start.map(|v| v.to_rfc3339());
        let cloud_sync_end = obj.cloud_sync_end.map(|v| v.to_rfc3339());
        let gcode_file_name = obj.gcode_file_name.clone();
        let recording_status = match obj.recording_status.as_ref() {
            "done" => Some(models::RecordingStatusEnum::Done),
            "progress" => Some(models::RecordingStatusEnum::Inprogress),
            "pending" => Some(models::RecordingStatusEnum::Pending),
            _ => None,
        };

        let cloud_sync_status = match obj.cloud_sync_status.as_ref() {
            "done" => Some(models::CloudSyncStatusEnum::Done),
            "progress" => Some(models::CloudSyncStatusEnum::Inprogress),
            "pending" => Some(models::CloudSyncStatusEnum::Pending),
            _ => None,
        };
        let request = models::PatchedVideoRecordingRequest {
            recording_start,
            recording_end,
            recording_status,
            cloud_sync_start,
            cloud_sync_end,
            cloud_sync_status,
            gcode_file_name,
        };
        let result =
            videos_api::video_recordings_partial_update(&self.reqwest_config(), id, Some(request))
                .await?;
        // save result locally
        let row = printnanny_edge_db::video_recording::UpdateVideoRecording {
            mp4_upload_url: Some(&result.mp4_upload_url),
            deleted: None,
            gcode_file_name: None,
            recording_status: None,
            recording_start: None,
            recording_end: None,
            mp4_download_url: None,
            cloud_sync_status: None,
            cloud_sync_start: None,
            cloud_sync_percent: None,
            cloud_sync_end: None,
        };
        printnanny_edge_db::video_recording::VideoRecording::update(
            &self.sqlite_connection,
            &obj.id,
            row,
        )?;
        Ok(result)
    }

    pub async fn video_recording_update_or_create(
        &self,
        obj: &printnanny_edge_db::video_recording::VideoRecording,
    ) -> Result<models::VideoRecording, VideoRecordingUpdateOrCreateError> {
        let id = obj.id.as_str();
        let recording_start = obj.recording_start.map(|v| v.to_rfc3339());
        let recording_end = obj.recording_end.map(|v| v.to_rfc3339());
        let cloud_sync_start = obj.cloud_sync_start.map(|v| v.to_rfc3339());
        let cloud_sync_end = obj.cloud_sync_end.map(|v| v.to_rfc3339());
        let gcode_file_name = obj.gcode_file_name.clone();
        let recording_status = match obj.recording_status.as_ref() {
            "done" => Some(models::RecordingStatusEnum::Done),
            "progress" => Some(models::RecordingStatusEnum::Inprogress),
            "pending" => Some(models::RecordingStatusEnum::Pending),
            _ => None,
        };

        let cloud_sync_status = match obj.cloud_sync_status.as_ref() {
            "done" => Some(models::CloudSyncStatusEnum::Done),
            "progress" => Some(models::CloudSyncStatusEnum::Inprogress),
            "pending" => Some(models::CloudSyncStatusEnum::Pending),
            _ => None,
        };

        let request = models::VideoRecordingRequest {
            recording_start,
            recording_end,
            recording_status,
            cloud_sync_start,
            cloud_sync_end,
            cloud_sync_status,
            gcode_file_name,
        };

        let result = videos_api::video_recordings_update_or_create(
            &self.reqwest_config(),
            id,
            Some(request),
        )
        .await?;
        // save result locally
        let row = printnanny_edge_db::video_recording::UpdateVideoRecording {
            mp4_upload_url: Some(&result.mp4_upload_url),
            deleted: None,
            gcode_file_name: None,
            recording_status: None,
            recording_start: None,
            recording_end: None,
            mp4_download_url: None,
            cloud_sync_status: None,
            cloud_sync_start: None,
            cloud_sync_percent: None,
            cloud_sync_end: None,
        };
        printnanny_edge_db::video_recording::VideoRecording::update(
            &self.sqlite_connection,
            &obj.id,
            row,
        )?;
        Ok(result)
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

impl From<&PrintNannySettings> for ApiService {
    fn from(settings: &PrintNannySettings) -> ApiService {
        ApiService::new(
            settings.cloud.clone(),
            settings.paths.db().display().to_string(),
        )
    }
}

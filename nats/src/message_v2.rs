use std::fmt::Debug;
use std::fs;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use printnanny_settings::cam::CameraVideoSource;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_dbus::printnanny_asyncapi_models::{self, CamerasLoadReply};
use printnanny_dbus::printnanny_asyncapi_models::{
    DeviceInfoLoadReply, PrintNannyCloudAuthReply, PrintNannyCloudAuthRequest, SettingsApp,
    SettingsApplyReply, SettingsApplyRequest, SettingsFile, SettingsLoadReply, SettingsRevertReply,
    SettingsRevertRequest, SystemdManagerDisableUnitsReply, SystemdManagerDisableUnitsRequest,
    SystemdManagerEnableUnitsReply, SystemdManagerEnableUnitsRequest,
    SystemdManagerGetUnitFileStateReply, SystemdManagerGetUnitFileStateRequest,
    SystemdManagerGetUnitReply, SystemdManagerGetUnitRequest, SystemdManagerRestartUnitReply,
    SystemdManagerRestartUnitRequest, SystemdManagerStartUnitReply, SystemdManagerStartUnitRequest,
    SystemdManagerStopUnitReply, SystemdManagerStopUnitRequest, SystemdUnitChange,
    SystemdUnitChangeState, SystemdUnitFileState,
};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use printnanny_settings::git2;
use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::vcs::VersionControlledSettings;

use printnanny_services::printnanny_api::ApiService;

#[async_trait]
pub trait NatsRequestHandler {
    type Request: Serialize + DeserializeOwned + Clone + Debug + NatsRequestHandler;
    type Reply: Serialize + DeserializeOwned + Clone + Debug;

    fn replace_subject_pattern(subject: &str, pattern: &str, replace: &str) -> String {
        subject.replace(pattern, replace)
    }
    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Request>;
    async fn handle(&self) -> Result<Self::Reply>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "subject_pattern")]
pub enum NatsRequest {
    // pi.{pi_id}.cameras.load
    #[serde(rename = "pi.{pi_id}.cameras.load")]
    CameraLoadRequest,

    // pi.{pi_id}.device_info.load
    #[serde(rename = "pi.{pi_id}.device_info.load")]
    DeviceInfoLoadRequest,

    // pi.{pi_id}.settings.*
    #[serde(rename = "pi.{pi_id}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthRequest(PrintNannyCloudAuthRequest),
    #[serde(rename = "pi.{pi_id}.settings.vcs.load")]
    SettingsLoadRequest,
    #[serde(rename = "pi.{pi_id}.settings.vcs.apply")]
    SettingsApplyRequest(SettingsApplyRequest),
    #[serde(rename = "pi.{pi_id}.settings.vcs.revert")]
    SettingsRevertRequest(SettingsRevertRequest),

    // pi.{pi_id}.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitsRequest(SystemdManagerDisableUnitsRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitsRequest(SystemdManagerEnableUnitsRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnit")]
    SystemdManagerGetUnitRequest(SystemdManagerGetUnitRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnitFileState")]
    SystemdManagerGetUnitFileStateRequest(SystemdManagerGetUnitFileStateRequest),
    // TODO: : Job type reload is not applicable for unit octoprint.service.
    // #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.ReloadUnit")]
    // SystemdManagerReloadUnitRequest(SystemdManagerReloadUnitRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitRequest(SystemdManagerRestartUnitRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitRequest(SystemdManagerStartUnitRequest),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitRequest(SystemdManagerStopUnitRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "subject_pattern")]
pub enum NatsReply {
    // pi.{pi_id}.cameras.load
    #[serde(rename = "pi.{pi_id}.cameras.load")]
    CameraLoadReply(CamerasLoadReply),

    // pi.{pi_id}.device_info.load
    #[serde(rename = "pi.{pi_id}.device_info.load")]
    DeviceInfoLoadReply(DeviceInfoLoadReply),

    // pi.{pi_id}.settings.*
    #[serde(rename = "pi.{pi_id}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.{pi_id}.settings.printnanny.load")]
    SettingsLoadReply(SettingsLoadReply),
    #[serde(rename = "pi.{pi_id}.settings.printnanny.apply")]
    SettingsApplyReply(SettingsApplyReply),
    #[serde(rename = "pi.{pi_id}.settings.printnanny.revert")]
    SettingsRevertReply(SettingsRevertReply),

    // pi.{pi_id}.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitsReply(SystemdManagerDisableUnitsReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitsReply(SystemdManagerEnableUnitsReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnit")]
    SystemdManagerGetUnitReply(SystemdManagerGetUnitReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnitFileState")]
    SystemdManagerGetUnitFileStateReply(SystemdManagerGetUnitFileStateReply),
    // TODO: : Job type reload is not applicable for unit octoprint.service.
    // #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.ReloadUnit")]
    // SystemdManagerReloadUnitReply(SystemdManagerReloadUnitReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitReply(SystemdManagerRestartUnitReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitReply(SystemdManagerStartUnitReply),
    #[serde(rename = "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitReply(SystemdManagerStopUnitReply),
}

impl NatsRequest {
    // message messages sent to: "pi.{pi_id}.device_info.load"
    pub async fn handle_device_info_load(&self) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let issue = fs::read_to_string(settings.paths.issue_txt)?;
        let os_release = fs::read_to_string(settings.paths.os_release)?;

        Ok(NatsReply::DeviceInfoLoadReply(DeviceInfoLoadReply {
            issue,
            os_release,
            printnanny_cli_version: "".into(), // TODO
            tailscale_address_ipv4: "".into(), // TODO
            tailscale_address_ipv6: "".into(), // TODO
        }))
    }

    // handle messages sent to: "pi.{pi_id}.settings.printnanny.cloud.auth"
    pub async fn handle_printnanny_cloud_auth(
        &self,
        request: &PrintNannyCloudAuthRequest,
    ) -> Result<NatsReply> {
        let api_service = ApiService::new()?;
        let result = api_service
            .connect_cloud_account(request.api_url.clone(), request.api_token.clone())
            .await;
        let result = match result {
            Ok(_) => NatsReply::PrintNannyCloudAuthReply(PrintNannyCloudAuthReply {
                status_code: 200,
                msg: format!("Success! Connected account: {}", request.email),
            }),
            Err(e) => NatsReply::PrintNannyCloudAuthReply(PrintNannyCloudAuthReply {
                status_code: 403,
                msg: format!("Error connecting account: {}", e),
            }),
        };
        Ok(result)
    }

    pub fn handle_cameras_load(&self) -> Result<NatsReply> {
        let cameras: Vec<printnanny_asyncapi_models::Camera> =
            CameraVideoSource::from_libcamera_list()?
                .iter()
                .map(|v| v.into())
                .collect();

        Ok(NatsReply::CameraLoadReply(
            printnanny_asyncapi_models::cameras_load_reply::CamerasLoadReply { cameras },
        ))
    }

    pub async fn handle_printnanny_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;

        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.git_revert_hooks(Some(oid)).await?;
        let files = vec![settings.to_payload(SettingsApp::Printnanny)?];
        self.build_settings_revert_reply(request, &settings, files)
    }

    async fn handle_octoprint_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.octoprint.git_revert_hooks(Some(oid)).await?;
        let files = vec![settings.octoprint.to_payload(SettingsApp::Octoprint)?];
        self.build_settings_revert_reply(request, &settings, files)
    }

    async fn handle_moonraker_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.moonraker.git_revert_hooks(Some(oid)).await?;
        let files = vec![settings.moonraker.to_payload(SettingsApp::Moonraker)?];
        self.build_settings_revert_reply(request, &settings, files)
    }

    async fn handle_klipper_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.klipper.git_revert_hooks(Some(oid)).await?;
        let files = vec![settings.klipper.to_payload(SettingsApp::Klipper)?];
        self.build_settings_revert_reply(request, &settings, files)
    }

    fn build_settings_revert_reply(
        &self,
        request: &SettingsRevertRequest,
        settings: &PrintNannySettings,
        files: Vec<SettingsFile>,
    ) -> Result<NatsReply> {
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        Ok(NatsReply::SettingsRevertReply(SettingsRevertReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        }))
    }

    async fn handle_printnanny_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;

        settings
            .save_and_commit(&request.file.content, Some(request.git_commit_msg.clone()))
            .await?;
        let file = settings.to_payload(SettingsApp::Printnanny)?;
        self.build_settings_apply_reply(request, settings, file)
    }

    async fn handle_octoprint_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        settings
            .octoprint
            .save_and_commit(&request.file.content, Some(request.git_commit_msg.clone()))
            .await?;
        let file = settings.octoprint.to_payload(SettingsApp::Octoprint)?;
        self.build_settings_apply_reply(request, settings, file)
    }

    async fn handle_moonraker_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        settings
            .moonraker
            .save_and_commit(&request.file.content, Some(request.git_commit_msg.clone()))
            .await?;
        let file = settings.moonraker.to_payload(SettingsApp::Moonraker)?;
        self.build_settings_apply_reply(request, settings, file)
    }

    async fn handle_klipper_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        settings
            .klipper
            .save_and_commit(&request.file.content, Some(request.git_commit_msg.clone()))
            .await?;
        let file = settings.klipper.to_payload(SettingsApp::Klipper)?;
        self.build_settings_apply_reply(request, settings, file)
    }

    fn build_settings_apply_reply(
        &self,
        _request: &SettingsApplyRequest,
        settings: PrintNannySettings,
        file: SettingsFile,
    ) -> Result<NatsReply> {
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        Ok(NatsReply::SettingsApplyReply(SettingsApplyReply {
            file: Box::new(file),
            git_head_commit,
            git_history,
        }))
    }

    fn handle_printnanny_settings_load(&self) -> Result<Vec<SettingsFile>> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.to_payload(SettingsApp::Printnanny)?];
        Ok(files)
    }

    fn handle_octoprint_settings_load(&self) -> Result<Vec<SettingsFile>> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.octoprint.to_payload(SettingsApp::Octoprint)?];
        Ok(files)
    }

    fn handle_moonraker_settings_load(&self) -> Result<Vec<SettingsFile>> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.moonraker.to_payload(SettingsApp::Moonraker)?];
        Ok(files)
    }

    fn handle_klipper_settings_load(&self) -> Result<Vec<SettingsFile>> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.klipper.to_payload(SettingsApp::Klipper)?];
        Ok(files)
    }

    pub fn handle_settings_load(&self) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;

        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();

        let mut files = self.handle_printnanny_settings_load()?;
        files.extend(self.handle_octoprint_settings_load()?);
        files.extend(self.handle_moonraker_settings_load()?);
        files.extend(self.handle_klipper_settings_load()?);
        Ok(NatsReply::SettingsLoadReply(SettingsLoadReply {
            files,
            git_head_commit,
            git_history,
        }))
    }

    pub async fn handle_settings_apply(&self, request: &SettingsApplyRequest) -> Result<NatsReply> {
        match *request.file.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_apply(request).await,
            SettingsApp::Octoprint => self.handle_octoprint_settings_apply(request).await,
            SettingsApp::Moonraker => self.handle_moonraker_settings_apply(request).await,
            SettingsApp::Klipper => self.handle_klipper_settings_apply(request).await,
        }
    }

    pub async fn handle_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_revert(request).await,
            SettingsApp::Octoprint => self.handle_octoprint_settings_revert(request).await,
            SettingsApp::Moonraker => self.handle_moonraker_settings_revert(request).await,
            SettingsApp::Klipper => self.handle_klipper_settings_revert(request).await,
        }
    }

    pub async fn handle_disable_units_request(
        &self,
        request: &SystemdManagerDisableUnitsRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let changes = proxy
            .disable_unit_files(request.files.clone(), false)
            .await?;
        let changes = changes
            .iter()
            .map(
                |(change_type, file, destination)| match change_type.as_str() {
                    "symlink" => SystemdUnitChange {
                        change: Box::new(SystemdUnitChangeState::Symlink),
                        file: file.to_string(),
                        destination: destination.to_string(),
                    },
                    "unlink" => SystemdUnitChange {
                        change: Box::new(SystemdUnitChangeState::Symlink),
                        file: file.to_string(),
                        destination: destination.to_string(),
                    },
                    _ => {
                        unimplemented!("No implementation for systemd change type {}", change_type)
                    }
                },
            )
            .collect();
        info!(
            "Disabled units: {:?} - changes: {:?}",
            request.files, changes
        );
        proxy.reload().await?;

        Ok(NatsReply::SystemdManagerDisableUnitsReply(
            SystemdManagerDisableUnitsReply { changes },
        ))
    }

    pub async fn handle_enable_units_request(
        &self,
        request: &SystemdManagerEnableUnitsRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let (_enablement_info, changes) = proxy
            .enable_unit_files(request.files.clone(), false, false)
            .await?;

        let changes = changes
            .iter()
            .map(
                |(change_type, file, destination)| match change_type.as_str() {
                    "symlink" => SystemdUnitChange {
                        change: Box::new(SystemdUnitChangeState::Symlink),
                        file: file.to_string(),
                        destination: destination.to_string(),
                    },
                    "unlink" => SystemdUnitChange {
                        change: Box::new(SystemdUnitChangeState::Symlink),
                        file: file.to_string(),
                        destination: destination.to_string(),
                    },
                    _ => {
                        unimplemented!("No implementation for systemd change type {}", change_type)
                    }
                },
            )
            .collect();
        info!(
            "Enabled units: {:?} - changes: {:?}",
            request.files, changes
        );
        proxy.reload().await?;

        Ok(NatsReply::SystemdManagerEnableUnitsReply(
            SystemdManagerEnableUnitsReply { changes },
        ))
    }

    async fn get_systemd_unit(
        &self,
        unit_name: String,
    ) -> Result<printnanny_asyncapi_models::SystemdUnit> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let unit_path = proxy.get_unit(unit_name.clone()).await?;
        let unit =
            printnanny_dbus::systemd1::models::SystemdUnit::from_owned_object_path(unit_path)
                .await?;
        let unit = printnanny_asyncapi_models::SystemdUnit::from(unit);
        Ok(unit)
    }

    async fn handle_get_unit_request(
        &self,
        request: &SystemdManagerGetUnitRequest,
    ) -> Result<NatsReply> {
        let unit = self.get_systemd_unit(request.unit_name.clone()).await?;
        Ok(NatsReply::SystemdManagerGetUnitReply(
            SystemdManagerGetUnitReply {
                unit: Box::new(unit),
            },
        ))
    }

    async fn handle_get_unit_file_state_request(
        &self,
        request: &SystemdManagerGetUnitFileStateRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;

        let unit_file_state = proxy.get_unit_file_state(request.unit_name.clone()).await?;

        let unit_file_state = match unit_file_state.as_str() {
            "enabled" => SystemdUnitFileState::Enabled,
            "enabled-runtime" => SystemdUnitFileState::EnabledMinusRuntime,
            "linked" => SystemdUnitFileState::Linked,
            "linked-runtime" => SystemdUnitFileState::LinkedMinusRuntime,
            "masked" => SystemdUnitFileState::Masked,
            "masked-runtime" => SystemdUnitFileState::MaskedMinusRuntime,
            "static" => SystemdUnitFileState::Static,
            "disabled" => SystemdUnitFileState::Disabled,
            "invalid" => SystemdUnitFileState::Invalid,
            _ => unimplemented!(),
        };

        Ok(NatsReply::SystemdManagerGetUnitFileStateReply(
            SystemdManagerGetUnitFileStateReply {
                unit_file_state: Box::new(unit_file_state),
            },
        ))
    }

    // TODO
    // Job type reload is not applicable for unit octoprint.service.
    // async fn handle_reload_unit_request(
    //     &self,
    //     request: &SystemdManagerReloadUnitRequest,
    // ) -> Result<NatsReply> {
    //     let connection = zbus::Connection::system().await?;
    //     let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
    //     let job = proxy
    //         .reload_unit(request.unit_name.clone(), "replace".into())
    //         .await?;
    //     let unit = self.get_systemd_unit(request.unit_name.clone()).await?;

    //     Ok(NatsReply::SystemdManagerReloadUnitReply(
    //         SystemdManagerReloadUnitReply {
    //             job: job.to_string(),
    //             unit: Box::new(unit),
    //         },
    //     ))
    // }

    async fn handle_restart_unit_request(
        &self,
        request: &SystemdManagerRestartUnitRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .restart_unit(request.unit_name.clone(), "replace".into())
            .await?;
        let unit = self.get_systemd_unit(request.unit_name.clone()).await?;

        Ok(NatsReply::SystemdManagerRestartUnitReply(
            SystemdManagerRestartUnitReply {
                job: job.to_string(),
                unit: Box::new(unit),
            },
        ))
    }

    async fn handle_start_unit_request(
        &self,
        request: &SystemdManagerStartUnitRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .start_unit(request.unit_name.clone(), "replace".into())
            .await?;
        let unit = self.get_systemd_unit(request.unit_name.clone()).await?;
        Ok(NatsReply::SystemdManagerStartUnitReply(
            SystemdManagerStartUnitReply {
                job: job.to_string(),
                unit: Box::new(unit),
            },
        ))
    }

    async fn handle_stop_unit_request(
        &self,
        request: &SystemdManagerStopUnitRequest,
    ) -> Result<NatsReply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .stop_unit(request.unit_name.clone(), "replace".into())
            .await?;
        let unit = self.get_systemd_unit(request.unit_name.clone()).await?;
        Ok(NatsReply::SystemdManagerStopUnitReply(
            SystemdManagerStopUnitReply {
                job: job.to_string(),
                unit: Box::new(unit),
            },
        ))
    }
}

#[async_trait]
impl NatsRequestHandler for NatsRequest {
    type Request = NatsRequest;
    type Reply = NatsReply;

    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Request> {
        match subject_pattern {
            "pi.{pi_id}.device_info.load" => Ok(NatsRequest::DeviceInfoLoadRequest),
            "pi.{pi_id}.settings.printnanny.cloud.auth" => {
                Ok(NatsRequest::PrintNannyCloudAuthRequest(
                    serde_json::from_slice::<PrintNannyCloudAuthRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.settings.vcs.load" => Ok(NatsRequest::SettingsLoadRequest),
            "pi.{pi_id}.settings.vcs.apply" => {
                Ok(NatsRequest::SettingsApplyRequest(serde_json::from_slice::<
                    SettingsApplyRequest,
                >(
                    payload.as_ref()
                )?))
            }
            "pi.{pi_id}.settings.vcs.revert" => Ok(NatsRequest::SettingsRevertRequest(
                serde_json::from_slice::<SettingsRevertRequest>(payload.as_ref())?,
            )),
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.DisableUnit" => {
                Ok(NatsRequest::SystemdManagerDisableUnitsRequest(
                    serde_json::from_slice::<SystemdManagerDisableUnitsRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.EnableUnit" => {
                Ok(NatsRequest::SystemdManagerEnableUnitsRequest(
                    serde_json::from_slice::<SystemdManagerEnableUnitsRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnit" => {
                Ok(NatsRequest::SystemdManagerGetUnitRequest(
                    serde_json::from_slice::<SystemdManagerGetUnitRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnitFileState" => {
                Ok(NatsRequest::SystemdManagerGetUnitFileStateRequest(
                    serde_json::from_slice::<SystemdManagerGetUnitFileStateRequest>(
                        payload.as_ref(),
                    )?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.RestartUnit" => {
                Ok(NatsRequest::SystemdManagerRestartUnitRequest(
                    serde_json::from_slice::<SystemdManagerRestartUnitRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StartUnit" => {
                Ok(NatsRequest::SystemdManagerStartUnitRequest(
                    serde_json::from_slice::<SystemdManagerStartUnitRequest>(payload.as_ref())?,
                ))
            }
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.StopUnit" => {
                Ok(NatsRequest::SystemdManagerStopUnitRequest(
                    serde_json::from_slice::<SystemdManagerStopUnitRequest>(payload.as_ref())?,
                ))
            }
            _ => Err(anyhow!(
                "NATS message handler not implemented for subject pattern {}",
                subject_pattern
            )),
        }
    }

    async fn handle(&self) -> Result<Self::Reply> {
        let reply = match self {
            // pi.{pi_id}.device_info.load
            NatsRequest::DeviceInfoLoadRequest => self.handle_device_info_load().await?,

            // pi.{pi_id}.settings.*
            NatsRequest::PrintNannyCloudAuthRequest(request) => {
                self.handle_printnanny_cloud_auth(request).await?
            }
            NatsRequest::SettingsLoadRequest => self.handle_settings_load()?,
            NatsRequest::SettingsApplyRequest(request) => {
                self.handle_settings_apply(request).await?
            }
            NatsRequest::SettingsRevertRequest(request) => {
                self.handle_settings_revert(request).await?
            }
            // pi.{pi_id}.dbus.org.freedesktop.systemd1.*
            NatsRequest::SystemdManagerDisableUnitsRequest(request) => {
                self.handle_disable_units_request(request).await?
            }
            NatsRequest::SystemdManagerEnableUnitsRequest(request) => {
                self.handle_enable_units_request(request).await?
            }
            NatsRequest::SystemdManagerGetUnitRequest(request) => {
                self.handle_get_unit_request(request).await?
            }
            NatsRequest::SystemdManagerGetUnitFileStateRequest(request) => {
                self.handle_get_unit_file_state_request(request).await?
            }
            NatsRequest::SystemdManagerRestartUnitRequest(request) => {
                self.handle_restart_unit_request(request).await?
            }
            NatsRequest::SystemdManagerStartUnitRequest(request) => {
                self.handle_start_unit_request(request).await?
            }
            NatsRequest::SystemdManagerStopUnitRequest(request) => {
                self.handle_stop_unit_request(request).await?
            }
        };

        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use tokio::runtime::Runtime;

    #[cfg(test)]
    fn make_settings_repo(jail: &mut figment::Jail) -> () {
        let output = jail.directory().to_str().unwrap();

        jail.create_file(
            "PrintNannySettingsTest.toml",
            &format!(
                r#"
            [paths]
            settings_dir = "{output}/settings"
            state_dir = "{output}/"
            log_dir = "{output}/log"
            "#,
                output = &output
            ),
        )
        .unwrap();
        jail.set_env("PRINTNANNY_SETTINGS", "PrintNannySettingsTest.toml");
        let settings = PrintNannySettings::new().unwrap();
        Runtime::new()
            .unwrap()
            .block_on(settings.init_local_git_repo(None))
            .unwrap();
    }

    #[test]
    fn test_replace_subject_pattern() {
        let subject = NatsRequest::replace_subject_pattern(
            "pi.localhost.dbus.org.freedesktop.systemd1.Manager.GetUnit",
            "localhost",
            "{pi_id}",
        );
        assert_eq!(
            subject,
            "pi.{pi_id}.dbus.org.freedesktop.systemd1.Manager.GetUnit"
        )
    }

    #[test(tokio::test)]
    async fn test_device_info_load() {
        let request = NatsRequest::DeviceInfoLoadRequest;

        let reply = request.handle().await.unwrap();
        if let NatsReply::DeviceInfoLoadReply(reply) = reply {
        } else {
            panic!("Expected NatsReply::DeviceInfoLoadReply")
        }
    }

    #[test]
    fn test_printnanny_cloud_auth_failed() {
        let email = "testing@test.com".to_string();
        let api_url = "http://localhost:8080/".to_string();
        let api_token = "test_token".to_string();
        let request = NatsRequest::PrintNannyCloudAuthRequest(PrintNannyCloudAuthRequest {
            email,
            api_url,
            api_token,
        });
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);
            let reply = Runtime::new().unwrap().block_on(request.handle()).unwrap();
            if let NatsReply::PrintNannyCloudAuthReply(reply) = reply {
                assert_eq!(reply.status_code, 403);
            } else {
                panic!("Expected NatsReply::PrintNannyCloudAuthReply")
            }
            Ok(())
        })
    }

    #[cfg(feature = "systemd")]
    #[test]
    fn test_printnanny_settings_apply_load_revert() {
        figment::Jail::expect_with(|jail| {
            // init git repo in jail tmp dir
            make_settings_repo(jail);

            // apply a settings change
            let mut settings = PrintNannySettings::new().unwrap();
            let original = settings.to_payload(SettingsApp::Printnanny).unwrap();
            let mut modified = original.clone();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            settings.paths.log_dir = "/path/to/testing".into();
            modified.content = settings.to_toml_string().unwrap();
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                file: Box::new(modified.clone()),
                git_head_commit,
                git_commit_msg: git_commit_msg.clone(),
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_apply.handle())
                .unwrap();
            let revert_commit = settings.get_git_head_commit().unwrap().oid;

            if let NatsReply::SettingsApplyReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                assert_eq!(reply.file.content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest;
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_load.handle())
                .unwrap();
            let reply = if let NatsReply::SettingsLoadReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                reply
            } else {
                panic!("Expected NatsReply::SettingsLoadReply")
            };

            // revert the settings
            let request_revert = NatsRequest::SettingsRevertRequest(SettingsRevertRequest {
                git_commit: revert_commit,
                app: Box::new(SettingsApp::Printnanny),
                files: reply.files,
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_revert.handle())
                .unwrap();
            if let NatsReply::SettingsRevertReply(reply) = reply {
                assert_eq!(reply.files[0].content, original.content);
            } else {
                panic!("Expected NatsReply::SettingsRevertReply")
            }

            Ok(())
        })
    }

    #[cfg(feature = "systemd")]
    #[test]
    fn test_octoprint_settings_apply_load_revert() {
        const OCTOPRINT_MODIFIED_SETTINGS: &str = r#"
        ---
        server:
          commands:
            systemShutdownCommand: sudo shutdown -h now
            systemRestartCommand: sudo shutdown -r now
            serverRestartCommand: sudo systemctl restart octoprint.service
        
        api:
          disabled: true
        
        system:
          actions:
            - name: Start PrintNanny Cam
              action: printnanny_cam_start
              command: sudo systemctl restart printnanny-vision.service
            - name: Stop PrintNanny Cam
              action: printnanny_cam_stop
              command: sudo systemctl stop printnanny-vision.service
        events:
          subscriptions:
            - command: sudo systemctl start printnanny-vision.service
              debug: false
              event: plugin_octoprint_nanny_vision_start
              type: system
              enabled: true
            - command: sudo systemctl stop printnanny-vision.service
              enabled: true
              debug: false
              event: plugin_octoprint_nanny_vision_stop
              type: system
        
        webcam:
          stream: /printnanny-hls/playlist.m3u8
        "#;
        figment::Jail::expect_with(|jail| {
            // init git repo in jail tmp dir
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();

            // apply a settings change
            let original = settings
                .octoprint
                .to_payload(SettingsApp::Octoprint)
                .unwrap();
            let mut modified = original.clone();
            modified.content = OCTOPRINT_MODIFIED_SETTINGS.into();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                file: Box::new(modified.clone()),
                git_head_commit,
                git_commit_msg: git_commit_msg.clone(),
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_apply.handle())
                .unwrap();
            let revert_commit = settings.get_git_head_commit().unwrap().oid;
            if let NatsReply::SettingsApplyReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                assert_eq!(reply.file.content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest;
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_load.handle())
                .unwrap();
            let reply = if let NatsReply::SettingsLoadReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                reply
            } else {
                panic!("Expected NatsReply::SettingsLoadReply")
            };

            // revert the settings
            let request_revert = NatsRequest::SettingsRevertRequest(SettingsRevertRequest {
                git_commit: revert_commit,
                app: Box::new(SettingsApp::Octoprint),
                files: reply.files,
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_revert.handle())
                .unwrap();
            if let NatsReply::SettingsRevertReply(reply) = reply {
                assert_eq!(reply.files[0].content, original.content);
            } else {
                panic!("Expected NatsReply::SettingsRevertReply")
            }

            Ok(())
        });
    }

    #[cfg(feature = "systemd")]
    #[test]
    fn test_moonraker_settings_apply_load_revert() {
        const MOONRAKER_MODIFIED_SETTINGS: &str = r#"
        # https://github.com/Arksine/moonraker/blob/master/docs/installation.md
        [server]
        host: 0.0.0.0
        port: 7125
        klippy_uds_address: /var/run/klipper/klippy.sock
        
        [machine]
        validate_service: false
        provider: systemd_dbus
        
        [authorization]
        cors_domains:
            https://my.mainsail.xyz
            http://my.mainsail.xyz
            http://*.local
            http://*.lan
        
        trusted_clients:
            10.0.0.0/8
            127.0.0.0/8
            169.254.0.0/16
            172.16.0.0/12
            192.168.0.0/16
            FE80::/10
            ::1/128
        
        # enables partial support of Octoprint API
        [octoprint_compat]
        
        # enables moonraker to track and store print history.
        [history]
        "#;
        figment::Jail::expect_with(|jail| {
            // init git repo in jail tmp dir
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();

            // apply a settings change
            let original = settings
                .moonraker
                .to_payload(SettingsApp::Octoprint)
                .unwrap();
            let mut modified = original.clone();
            modified.content = MOONRAKER_MODIFIED_SETTINGS.into();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                file: Box::new(modified.clone()),
                git_head_commit,
                git_commit_msg: git_commit_msg.clone(),
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_apply.handle())
                .unwrap();
            let revert_commit = settings.get_git_head_commit().unwrap().oid;
            if let NatsReply::SettingsApplyReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                assert_eq!(reply.file.content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest;
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_load.handle())
                .unwrap();
            let reply = if let NatsReply::SettingsLoadReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                reply
            } else {
                panic!("Expected NatsReply::SettingsLoadReply")
            };

            // revert the settings
            let request_revert = NatsRequest::SettingsRevertRequest(SettingsRevertRequest {
                git_commit: revert_commit,
                app: Box::new(SettingsApp::Moonraker),
                files: reply.files,
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_revert.handle())
                .unwrap();
            if let NatsReply::SettingsRevertReply(reply) = reply {
                assert_eq!(reply.files[0].content, original.content);
            } else {
                panic!("Expected NatsReply::SettingsRevertReply")
            }

            Ok(())
        });
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_get_unit_file_state_ok() {
        let request = NatsRequest::SystemdManagerGetUnitFileStateRequest(
            SystemdManagerGetUnitFileStateRequest {
                unit_name: "octoprint.service".into(),
            },
        );
        let reply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerGetUnitFileStateReply(reply) = reply {
            // unit may already be in an enabled state
            assert!(
                *reply.unit_file_state == SystemdUnitFileState::Enabled
                    || *reply.unit_file_state == SystemdUnitFileState::Disabled
            );
        } else {
            panic!("Expected NatsReply::SystemdManagerGetUnit")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_get_unit_file_state_error() {
        let request = NatsRequest::SystemdManagerGetUnitFileStateRequest(
            SystemdManagerGetUnitFileStateRequest {
                unit_name: "doesnotexist.service".into(),
            },
        );
        let reply = request.handle().await;
        assert!(reply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_enable_disable_unit_ok() {
        let request =
            NatsRequest::SystemdManagerEnableUnitsRequest(SystemdManagerEnableUnitsRequest {
                files: vec!["octoprint.service".into()],
            });
        let natsreply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerEnableUnitsReply(reply) = natsreply {
            // unit may already be in an enabled state
            assert!(reply.changes.len() == 1 || reply.changes.len() == 0);
        } else {
            panic!("Expected NatsReply::SystemdManagerEnableUnitReply")
        }

        let request =
            NatsRequest::SystemdManagerDisableUnitsRequest(SystemdManagerDisableUnitsRequest {
                files: vec!["octoprint.service".into()],
            });
        let natsreply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerDisableUnitsReply(reply) = natsreply {
            // unit is guaranteed to be in enabled state from prior request
            assert_eq!(reply.changes.len(), 1);
        } else {
            panic!("Expected NatsReply::SystemdManagerDisableUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_disable_unit_error() {
        let request = SystemdManagerDisableUnitsRequest {
            files: vec!["doesnotexist.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerDisableUnitsRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_enable_unit_error() {
        let request = SystemdManagerEnableUnitsRequest {
            files: vec!["doesnotexist.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerEnableUnitsRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_get_unit_error() {
        let request = NatsRequest::SystemdManagerGetUnitRequest(SystemdManagerGetUnitRequest {
            unit_name: "doesnotexist.service".into(),
        });
        let reply = request.handle().await;
        assert!(reply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_restart_unit_error() {
        let request =
            NatsRequest::SystemdManagerRestartUnitRequest(SystemdManagerRestartUnitRequest {
                unit_name: "doesnotexist.service".into(),
            });
        let reply = request.handle().await;
        assert!(reply.is_err());
    }
    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_reload_unit_ok() {
        let request =
            NatsRequest::SystemdManagerRestartUnitRequest(SystemdManagerRestartUnitRequest {
                unit_name: "octoprint.service".into(),
            });
        let reply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerRestartUnitReply(reply) = reply {
            assert_eq!(
                *(*reply.unit).load_state,
                printnanny_asyncapi_models::SystemdUnitLoadState::Loaded
            );
        } else {
            panic!("Expected NatsReply::SystemdManagerRestartUniReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_start_unit_error() {
        let request = NatsRequest::SystemdManagerStartUnitRequest(SystemdManagerStartUnitRequest {
            unit_name: "doesnotexist.service".into(),
        });
        let reply = request.handle().await;
        assert!(reply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_start_unit_ok() {
        let request = NatsRequest::SystemdManagerStartUnitRequest(SystemdManagerStartUnitRequest {
            unit_name: "octoprint.service".into(),
        });
        let reply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerStartUnitReply(reply) = reply {
            assert_eq!(
                *(*reply.unit).load_state,
                printnanny_asyncapi_models::SystemdUnitLoadState::Loaded
            );
        } else {
            panic!("Expected NatsReply::SystemdManagerStartUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_stop_unit_error() {
        let request = NatsRequest::SystemdManagerStopUnitRequest(SystemdManagerStopUnitRequest {
            unit_name: "doesnotexist.service".into(),
        });
        let reply = request.handle().await;
        assert!(reply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_stop_unit_ok() {
        let request =
            NatsRequest::SystemdManagerEnableUnitsRequest(SystemdManagerEnableUnitsRequest {
                files: vec!["octoprint.service".into()],
            });
        let natsreply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerEnableUnitsReply(reply) = natsreply {
            // unit may already be in an enabled state
            assert!(reply.changes.len() == 1 || reply.changes.len() == 0);
        } else {
            panic!("Expected NatsReply::SystemdManagerEnableUnitReply")
        }
        request.handle().await.unwrap();

        let request = NatsRequest::SystemdManagerStopUnitRequest(SystemdManagerStopUnitRequest {
            unit_name: "octoprint.service".into(),
        });
        let reply = request.handle().await.unwrap();
        if let NatsReply::SystemdManagerStopUnitReply(reply) = reply {
            assert_eq!(
                *(*reply.unit).load_state,
                printnanny_asyncapi_models::SystemdUnitLoadState::Loaded
            );
        } else {
            panic!("Expected NatsReply::SystemdManagerStopUnitReply")
        }
    }
}

use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use log::info;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_dbus::systemd1::models::SystemdUnit;
use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use printnanny_services::git2;
use printnanny_services::settings::{PrintNannySettings, SettingsFormat};
use printnanny_services::vcs::{GitCommit, VersionControlledSettings};

#[async_trait]
pub trait NatsRequestReplyHandler {
    type Request: Serialize + DeserializeOwned + Clone + Debug;
    type Reply: Serialize + DeserializeOwned + Clone + Debug;
    async fn handle(&self) -> Result<Self::Reply>;
}

// pi.dbus.org.freedesktop.systemd1.Manager.GetUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerGetUnitRequest {
    name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerGetUnitReply {
    request: SystemdManagerGetUnitRequest,
    unit: printnanny_dbus::systemd1::models::SystemdUnit,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerGetUnitRequest {
    type Request = SystemdManagerGetUnitRequest;
    type Reply = SystemdManagerGetUnitReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let unit_path = proxy.get_unit(self.name.clone()).await?;

        let unit = SystemdUnit::from_owned_object_path(unit_path).await?;
        let reply = Self::Reply {
            unit,
            request: self.clone(),
        };
        Ok(reply)
    }
}

// pi.dbus.org.freedesktop.systemd1.Manager.StartUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerStartUnitRequest {
    name: String,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerStartUnitReply {
    request: SystemdManagerStartUnitRequest,
    job: zbus::zvariant::OwnedObjectPath,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerStartUnitRequest {
    type Request = SystemdManagerStartUnitRequest;
    type Reply = SystemdManagerStartUnitReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .start_unit(self.name.clone(), "replace".into())
            .await?;
        let reply = Self::Reply {
            job,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.RestartUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerRestartUnitRequest {
    name: String,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerRestartUnitReply {
    request: SystemdManagerRestartUnitRequest,
    job: zbus::zvariant::OwnedObjectPath,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerRestartUnitRequest {
    type Request = SystemdManagerRestartUnitRequest;
    type Reply = SystemdManagerRestartUnitReply;
    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .restart_unit(self.name.clone(), "replace".into())
            .await?;
        let reply = Self::Reply {
            job,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.StopUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerStopUnitRequest {
    name: String,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerStopUnitReply {
    request: SystemdManagerStopUnitRequest,
    job: zbus::zvariant::OwnedObjectPath,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerStopUnitRequest {
    type Request = SystemdManagerStopUnitRequest;
    type Reply = SystemdManagerStopUnitReply;
    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy.stop_unit(self.name.clone(), "replace".into()).await?;
        let reply = Self::Reply {
            job,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.EnableUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerEnableUnitRequest {
    files: Vec<String>,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerEnableUnitReply {
    request: SystemdManagerEnableUnitRequest,
    changes: Vec<(String, String, String)>,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerEnableUnitRequest {
    type Request = SystemdManagerEnableUnitRequest;
    type Reply = SystemdManagerEnableUnitReply;
    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let (_enablement_info, changes) = proxy
            .enable_unit_files(self.files.clone(), false, false)
            .await?;
        let reply = Self::Reply {
            changes,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.DisableUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerDisableUnitRequest {
    files: Vec<String>,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerDisableUnitReply {
    request: SystemdManagerDisableUnitRequest,
    changes: Vec<(String, String, String)>,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerDisableUnitRequest {
    type Request = SystemdManagerDisableUnitRequest;
    type Reply = SystemdManagerDisableUnitReply;
    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let changes = proxy.disable_unit_files(self.files.clone(), false).await?;
        let reply = Self::Reply {
            changes,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.ReloadUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerReloadUnitRequest {
    name: String, // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerReloadUnitReply {
    request: SystemdManagerReloadUnitRequest,
    job: zbus::zvariant::OwnedObjectPath,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerReloadUnitRequest {
    type Request = SystemdManagerReloadUnitRequest;
    type Reply = SystemdManagerReloadUnitReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .restart_unit(self.name.clone(), "replace".into())
            .await?;
        let reply = Self::Reply {
            job,
            request: self.clone(),
        };
        Ok(reply)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectPrintNannyCloudRequest {
    email: String,
    api_token: String,
    api_uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectPrintNannyCloudReply {
    request: ConnectPrintNannyCloudRequest,
    detail: String,
}

#[async_trait]
impl NatsRequestReplyHandler for ConnectPrintNannyCloudRequest {
    type Request = ConnectPrintNannyCloudRequest;
    type Reply = ConnectPrintNannyCloudReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let settings = PrintNannySettings::new()?;
        settings
            .connect_cloud_account(self.api_uri.clone(), self.api_token.clone())
            .await?;

        let res = Self::Reply {
            request: self.clone(),
            detail: format!(
                "Success! Connected PrintNanny Cloud account belonging to {}",
                self.email
            ),
        };
        Ok(res)
    }
}

//  pi.settings.gst_pipeline.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsLoadRequest {
    format: SettingsFormat,
}

//  pi.settings.gst_pipeline.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsLoadReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for GstPipelineSettingsLoadRequest {
    type Request = GstPipelineSettingsLoadRequest;
    type Reply = GstPipelineSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.gst_pipeline.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsApplyRequest {
    parent_commit: String,
    format: SettingsFormat,
}

//  pi.settings.gst_pipeline.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsApplyReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
    commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for GstPipelineSettingsApplyRequest {
    type Request = GstPipelineSettingsLoadRequest;
    type Reply = GstPipelineSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.gst_pipeline.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsRevertRequest {
    commit: String,
}

//  pi.settings.gst_pipeline.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineSettingsRevertReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for GstPipelineSettingsRevertRequest {
    type Request = GstPipelineSettingsLoadRequest;
    type Reply = GstPipelineSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.moonraker.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsLoadRequest {
    format: SettingsFormat,
}

//  pi.settings.moonraker.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsLoadReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for MoonrakerSettingsLoadRequest {
    type Request = MoonrakerSettingsLoadRequest;
    type Reply = MoonrakerSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.moonraker.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsApplyRequest {
    parent_commit: String,
    format: SettingsFormat,
}

//  pi.settings.moonraker.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsApplyReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
    commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for MoonrakerSettingsApplyRequest {
    type Request = MoonrakerSettingsLoadRequest;
    type Reply = MoonrakerSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.moonraker.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsRevertRequest {
    commit: String,
}

//  pi.settings.moonraker.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerSettingsRevertReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for MoonrakerSettingsRevertRequest {
    type Request = MoonrakerSettingsLoadRequest;
    type Reply = MoonrakerSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.klipper.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsLoadRequest {
    format: SettingsFormat,
}

//  pi.settings.klipper.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsLoadReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for KlipperSettingsLoadRequest {
    type Request = KlipperSettingsLoadRequest;
    type Reply = KlipperSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.klipper.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsApplyRequest {
    parent_commit: String,
    format: SettingsFormat,
}

//  pi.settings.klipper.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsApplyReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
    commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for KlipperSettingsApplyRequest {
    type Request = KlipperSettingsLoadRequest;
    type Reply = KlipperSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.klipper.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsRevertRequest {
    commit: String,
}

//  pi.settings.klipper.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KlipperSettingsRevertReply {
    data: String,
    format: SettingsFormat,
    parent_commit: String,
}

#[async_trait]
impl NatsRequestReplyHandler for KlipperSettingsRevertRequest {
    type Request = KlipperSettingsLoadRequest;
    type Reply = KlipperSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        todo!()
    }
}

//  pi.settings.gst_pipeline.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsLoadRequest {
    format: SettingsFormat,
}

//  pi.settings.octoprint.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsLoadReply {
    request: OctoPrintSettingsLoadRequest,
    filename: String,
    contents: String,
    commit: GitCommit,
    format: SettingsFormat,
}

#[async_trait]
impl NatsRequestReplyHandler for OctoPrintSettingsLoadRequest {
    type Request = OctoPrintSettingsLoadRequest;
    type Reply = OctoPrintSettingsLoadReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let settings = PrintNannySettings::new()?;

        let commit = settings.octoprint.get_git_head_commit()?;
        let contents = settings.octoprint.read_settings()?;
        let filename = settings.octoprint.settings_file.display().to_string();

        Ok(Self::Reply {
            request: self.clone(),
            commit,
            contents,
            filename,
            format: SettingsFormat::Yaml,
        })
    }
}

//  pi.settings.octoprint.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsApplyRequest {
    filename: String,
    contents: String,
    parent: String,
    format: SettingsFormat,
}

//  pi.settings.octoprint.apply
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsApplyReply {
    request: OctoPrintSettingsApplyRequest,
    format: SettingsFormat,
    filename: String,
    contents: String,
    git_commit: GitCommit,
    git_history: Vec<GitCommit>,
}

#[async_trait]
impl NatsRequestReplyHandler for OctoPrintSettingsApplyRequest {
    type Request = OctoPrintSettingsApplyRequest;
    type Reply = OctoPrintSettingsApplyReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let settings = PrintNannySettings::new()?;
        settings.octoprint.save(&self.contents, None).await?;
        let git_commit = settings.octoprint.get_git_head_commit()?;
        let contents = settings.octoprint.read_settings()?;
        let filename = settings.octoprint.settings_file.display().to_string();
        let git_history = settings.octoprint.get_rev_list()?;

        Ok(Self::Reply {
            request: self.clone(),
            git_history,
            git_commit,
            contents,
            filename,
            format: SettingsFormat::Yaml,
        })
    }
}

//  pi.settings.octoprint.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsRevertRequest {
    git_commit: String,
}

//  pi.settings.octoprint.revert
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintSettingsRevertReply {
    request: OctoPrintSettingsRevertRequest,
    filename: String,
    contents: String,
    git_commit: GitCommit,
    git_history: Vec<GitCommit>,
    format: SettingsFormat,
}

#[async_trait]
impl NatsRequestReplyHandler for OctoPrintSettingsRevertRequest {
    type Request = OctoPrintSettingsRevertRequest;
    type Reply = OctoPrintSettingsRevertReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let settings = PrintNannySettings::new()?;
        let oid = git2::Oid::from_str(&self.git_commit)?;
        settings.octoprint.git_revert(Some(oid))?;
        let git_commit = settings.octoprint.get_git_head_commit()?;
        let contents = settings.octoprint.read_settings()?;
        let filename = settings.octoprint.settings_file.display().to_string();
        let git_history = settings.octoprint.get_rev_list()?;

        Ok(Self::Reply {
            git_commit,
            git_history,
            contents,
            filename,
            request: self.clone(),
            format: SettingsFormat::Yaml,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    // pi.command.*
    #[serde(rename = "pi.command.connect_printnanny_cloud_account")]
    ConnectPrintNannyCloudRequest(ConnectPrintNannyCloudRequest),

    // pi.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitRequest(SystemdManagerDisableUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitRequest(SystemdManagerEnableUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.GetUnit")]
    SystemdManagerGetUnitRequest(SystemdManagerGetUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.ReloadUnit")]
    SystemdManagerReloadUnitRequest(SystemdManagerReloadUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitRequest(SystemdManagerRestartUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitRequest(SystemdManagerStartUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitRequest(SystemdManagerStopUnitRequest),

    // pi.settings.*
    #[serde(rename = "pi.settings.gst_pipeline.load")]
    GstPipelineSettingsLoadRequest(GstPipelineSettingsLoadRequest),
    #[serde(rename = "pi.settings.gst_pipeline.apply")]
    GstPipelineSettingsApplyRequest(GstPipelineSettingsApplyRequest),
    #[serde(rename = "pi.settings.gst_pipeline.revert")]
    GstPipelineSettingsRevertRequest(GstPipelineSettingsRevertRequest),

    #[serde(rename = "pi.settings.klipper.load")]
    KlipperSettingsLoadRequest(KlipperSettingsLoadRequest),
    #[serde(rename = "pi.settings.klipper.apply")]
    KlipperSettingsApplyRequest(KlipperSettingsApplyRequest),
    #[serde(rename = "pi.settings.klipper.revert")]
    KlipperSettingsRevertRequest(KlipperSettingsRevertRequest),

    #[serde(rename = "pi.settings.moonraker.load")]
    MoonrakerSettingsLoadRequest(MoonrakerSettingsLoadRequest),
    #[serde(rename = "pi.settings.moonraker.apply")]
    MoonrakerSettingsApplyRequest(MoonrakerSettingsApplyRequest),
    #[serde(rename = "pi.settings.moonraker.revert")]
    MoonrakerSettingsRevertRequest(MoonrakerSettingsRevertRequest),

    #[serde(rename = "pi.settings.octoprint.load")]
    OctoPrintSettingsLoadRequest(OctoPrintSettingsLoadRequest),
    #[serde(rename = "pi.settings.octoprint.apply")]
    OctoPrintSettingsApplyRequest(OctoPrintSettingsApplyRequest),
    #[serde(rename = "pi.settings.octoprint.revert")]
    OctoPrintSettingsRevertRequest(OctoPrintSettingsRevertRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsReply {
    // pi.command.*
    #[serde(rename = "pi.command.connect_printnanny_cloud_account")]
    ConnectPrintNannyCloudReply(SystemdManagerStopUnitReply),

    // pi.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitReply(SystemdManagerDisableUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitReply(SystemdManagerEnableUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.GetUnit")]
    SystemdManagerGetUnitReply(SystemdManagerGetUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.ReloadUnit")]
    SystemdManagerReloadUnitReply(SystemdManagerReloadUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitReply(SystemdManagerRestartUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitReply(SystemdManagerStartUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitReply(SystemdManagerStopUnitReply),

    // pi.settings.*
    #[serde(rename = "pi.settings.gst_pipeline.load")]
    GstPipelineSettingsLoadReply(GstPipelineSettingsLoadReply),
    #[serde(rename = "pi.settings.gst_pipeline.apply")]
    GstPipelineSettingsApplyReply(GstPipelineSettingsApplyReply),
    #[serde(rename = "pi.settings.gst_pipeline.revert")]
    GstPipelineSettingsRevertReply(GstPipelineSettingsRevertReply),

    #[serde(rename = "pi.settings.klipper.load")]
    KlipperSettingsLoadReply(KlipperSettingsLoadReply),
    #[serde(rename = "pi.settings.klipper.apply")]
    KlipperSettingsApplyReply(KlipperSettingsApplyReply),
    #[serde(rename = "pi.settings.klipper.revert")]
    KlipperSettingsRevertReply(KlipperSettingsRevertReply),

    #[serde(rename = "pi.settings.moonraker.load")]
    MoonrakerSettingsLoadReply(MoonrakerSettingsLoadReply),
    #[serde(rename = "pi.settings.moonraker.apply")]
    MoonrakerSettingsApplyReply(MoonrakerSettingsApplyReply),
    #[serde(rename = "pi.settings.moonraker.revert")]
    MoonrakerSettingsRevertReply(MoonrakerSettingsRevertReply),

    #[serde(rename = "pi.settings.octoprint.load")]
    OctoPrintSettingsLoadReply(OctoPrintSettingsLoadReply),
    #[serde(rename = "pi.settings.octoprint.apply")]
    OctoPrintSettingsApplyReply(OctoPrintSettingsApplyReply),
    #[serde(rename = "pi.settings.octoprint.revert")]
    OctoPrintSettingsRevertReply(OctoPrintSettingsRevertReply),
}

//  pi.settings.octoprint.load
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NatsError<T> {
    request: T,
    error: String,
}

#[async_trait]
impl NatsRequestReplyHandler for NatsRequest {
    type Request = NatsRequest;
    type Reply = NatsReply;

    async fn handle(&self) -> Result<NatsReply> {
        let reply = match self {
            NatsRequest::SystemdManagerGetUnitRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::SystemdManagerGetUnitReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::SystemdManagerDisableUnitRequest(request) => {
                match request.handle().await {
                    Ok(r) => Ok(NatsReply::SystemdManagerDisableUnitReply(r)),
                    Err(e) => Err(e),
                }
            }
            NatsRequest::SystemdManagerEnableUnitRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::SystemdManagerEnableUnitReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::SystemdManagerReloadUnitRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::SystemdManagerReloadUnitReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::SystemdManagerRestartUnitRequest(request) => {
                match request.handle().await {
                    Ok(r) => Ok(NatsReply::SystemdManagerRestartUnitReply(r)),
                    Err(e) => Err(e),
                }
            }
            NatsRequest::SystemdManagerStartUnitRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::SystemdManagerStartUnitReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::SystemdManagerStopUnitRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::SystemdManagerStopUnitReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::ConnectPrintNannyCloudRequest(_) => todo!(),
            NatsRequest::GstPipelineSettingsLoadRequest(_) => todo!(),
            NatsRequest::GstPipelineSettingsApplyRequest(_) => todo!(),
            NatsRequest::GstPipelineSettingsRevertRequest(_) => todo!(),
            NatsRequest::KlipperSettingsLoadRequest(_) => todo!(),
            NatsRequest::KlipperSettingsApplyRequest(_) => todo!(),
            NatsRequest::KlipperSettingsRevertRequest(_) => todo!(),
            NatsRequest::MoonrakerSettingsLoadRequest(_) => todo!(),
            NatsRequest::MoonrakerSettingsApplyRequest(_) => todo!(),
            NatsRequest::MoonrakerSettingsRevertRequest(_) => todo!(),
            NatsRequest::OctoPrintSettingsLoadRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::OctoPrintSettingsLoadReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::OctoPrintSettingsApplyRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::OctoPrintSettingsApplyReply(r)),
                Err(e) => Err(e),
            },
            NatsRequest::OctoPrintSettingsRevertRequest(request) => match request.handle().await {
                Ok(r) => Ok(NatsReply::OctoPrintSettingsRevertReply(r)),
                Err(e) => Err(e),
            },
        };

        info!("Sending NatsReply: {:?}", reply);
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use test_log::test;
    use tokio::runtime::Runtime;

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

    fn make_settings_repo(jail: &mut figment::Jail) -> () {
        let output = jail.directory().to_str().unwrap();

        jail.create_file(
            "PrintNannySettingsTest.toml",
            &format!(
                r#"
            [paths]
            settings_dir = "{output}/settings"
            log_dir = "{output}/log"
            "#,
                output = &output
            ),
        )
        .unwrap();
        jail.set_env("PRINTNANNY_SETTINGS", "PrintNannySettingsTest.toml");
        let settings = PrintNannySettings::new().unwrap();
        settings.octoprint.git_clone().unwrap();
        settings.octoprint.init_local_git_config().unwrap();
    }

    #[test]
    fn test_load_octoprint_settings() {
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);
            let settings = PrintNannySettings::new().unwrap();
            let expected =
                fs::read_to_string(settings.paths.settings_dir.join("octoprint/octoprint.yaml"))
                    .unwrap();

            let request = OctoPrintSettingsLoadRequest {
                format: SettingsFormat::Yaml,
            };

            let natsrequest = NatsRequest::OctoPrintSettingsLoadRequest(request.clone());
            let natsreply = Runtime::new()
                .unwrap()
                .block_on(natsrequest.handle())
                .unwrap();
            if let NatsReply::OctoPrintSettingsLoadReply(reply) = natsreply {
                assert_eq!(reply.request, request);
                assert_eq!(reply.contents, expected);
            }
            Ok(())
        })
    }

    #[test]
    #[cfg(feature = "systemd")]
    fn test_apply_octoprint_settings() {
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();

            let head = settings.octoprint.get_git_head_commit().unwrap();
            let git_history = settings.octoprint.get_rev_list().unwrap();

            let request = OctoPrintSettingsApplyRequest {
                format: SettingsFormat::Yaml,
                filename: settings.octoprint.get_settings_file().display().to_string(),
                contents: OCTOPRINT_MODIFIED_SETTINGS.to_string(),
                parent: head.oid,
            };

            let natsrequest = NatsRequest::OctoPrintSettingsApplyRequest(request.clone());
            let natsreply = Runtime::new()
                .unwrap()
                .block_on(natsrequest.handle())
                .unwrap();
            if let NatsReply::OctoPrintSettingsApplyReply(reply) = natsreply {
                assert_eq!(reply.request, request);
                assert_eq!(&reply.contents, OCTOPRINT_MODIFIED_SETTINGS);

                let settings = PrintNannySettings::new().unwrap();
                assert_eq!(reply.contents, settings.octoprint.read_settings().unwrap());
                assert_eq!(reply.git_history.len(), git_history.len() + 1);
            }
            Ok(())
        })
    }

    #[test]
    #[cfg(feature = "systemd")]
    fn test_revert_octoprint_settings() {
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();
            let before =
                fs::read_to_string(settings.paths.settings_dir.join("octoprint/octoprint.yaml"))
                    .unwrap();
            Runtime::new()
                .unwrap()
                .block_on(settings.octoprint.save(
                    &OCTOPRINT_MODIFIED_SETTINGS.to_string(),
                    Some("Test modify octoprint.yaml".to_string()),
                ))
                .unwrap();
            let commit = settings.octoprint.get_git_head_commit().unwrap();

            let request = OctoPrintSettingsRevertRequest {
                git_commit: commit.oid,
            };

            let natsrequest = NatsRequest::OctoPrintSettingsRevertRequest(request.clone());
            let natsreply = Runtime::new()
                .unwrap()
                .block_on(natsrequest.handle())
                .unwrap();
            if let NatsReply::OctoPrintSettingsRevertReply(reply) = natsreply {
                assert_eq!(reply.request, request);
                assert_eq!(reply.contents, before);
                let settings = PrintNannySettings::new().unwrap();
                assert_eq!(reply.contents, settings.octoprint.read_settings().unwrap());
            }
            Ok(())
        })
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_disable_unit_ok() {
        let request = SystemdManagerDisableUnitRequest {
            files: vec!["octoprint.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerDisableUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerDisableUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
        } else {
            panic!("Expected NatsReply::SystemdManagerDisableUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_disable_unit_error() {
        let request = SystemdManagerDisableUnitRequest {
            files: vec!["doesnotexist.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerDisableUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_enable_unit_ok() {
        let request = SystemdManagerEnableUnitRequest {
            files: vec!["octoprint.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerEnableUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerEnableUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
        } else {
            panic!("Expected NatsReply::SystemdManagerEnableUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_enable_unit_error() {
        let request = SystemdManagerEnableUnitRequest {
            files: vec!["doesnotexist.service".into()],
        };
        let natsrequest = NatsRequest::SystemdManagerEnableUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_get_unit() {
        use printnanny_dbus::systemd1::models::SystemdUnitFileState;

        let request = SystemdManagerGetUnitRequest {
            name: "octoprint.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerGetUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerGetUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
            assert_eq!(reply.unit.unit_file_state, SystemdUnitFileState::Enabled);
        } else {
            panic!("Expected NatsReply::SystemdManagerGetUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_start_unit_ok() {
        let request = SystemdManagerStartUnitRequest {
            name: "octoprint.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerStartUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerStartUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
        } else {
            panic!("Expected NatsReply::SystemdManagerStartUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_start_unit_error() {
        let request = SystemdManagerStartUnitRequest {
            name: "doesnotexist.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerStartUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_restart_unit_ok() {
        let request = SystemdManagerRestartUnitRequest {
            name: "octoprint.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerRestartUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerRestartUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
        } else {
            panic!("Expected NatsReply::SystemdManagerRestartUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_restart_unit_error() {
        let request = SystemdManagerRestartUnitRequest {
            name: "doesnotexist.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerRestartUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_stop_unit_ok() {
        let request = SystemdManagerStopUnitRequest {
            name: "octoprint.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerStopUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerStopUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
        } else {
            panic!("Expected NatsReply::SystemdManagerStopUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_stop_unit_error() {
        let request = SystemdManagerStopUnitRequest {
            name: "doesnotexist.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerStopUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_reload_unit_ok() {
        let request = SystemdManagerReloadUnitRequest {
            name: "octoprint.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerReloadUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await.unwrap();
        if let NatsReply::SystemdManagerReloadUnitReply(reply) = natsreply {
            assert_eq!(reply.request, request);
            // assert_eq!(reply.status, "ok")
        } else {
            panic!("Expected NatsReply::SystemdManagerReloadUnitReply")
        }
    }

    #[cfg(feature = "systemd")]
    #[test(tokio::test)] // async test
    async fn test_dbus_systemd_manager_reload_unit_error() {
        let request = SystemdManagerReloadUnitRequest {
            name: "doesnotexist.service".into(),
        };
        let natsrequest = NatsRequest::SystemdManagerReloadUnitRequest(request.clone());
        let natsreply = natsrequest.handle().await;
        assert!(natsreply.is_err());
    }
}

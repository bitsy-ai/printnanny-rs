use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_dbus;
use printnanny_dbus::zbus;

#[async_trait]
pub trait NatsRequestReplyHandler {
    type Request: Serialize + DeserializeOwned + Clone + Debug;
    type Reply: Serialize + DeserializeOwned + Clone + Debug;
    async fn handle(&self) -> Result<Self::Reply>;
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
        let proxy = printnanny_dbus::systemd1::manager::ManagerProxy::new(&connection).await?;
        let job = proxy.start_unit(&self.name, "replace").await?;
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
        let proxy = printnanny_dbus::systemd1::manager::ManagerProxy::new(&connection).await?;
        let job = proxy.restart_unit(&self.name, "replace").await?;
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
        let proxy = printnanny_dbus::systemd1::manager::ManagerProxy::new(&connection).await?;
        let job = proxy.stop_unit(&self.name, "replace").await?;
        let reply = Self::Reply {
            job,
            request: self.clone(),
        };
        Ok(reply)
    }
}

//  pi.dbus.org.freedesktop.systemd1.Manager.StopUnit
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerEnableUnitRequest {
    files: Vec<String>,
    // mode: String, // "replace", "fail", "isolate", "ignore-dependencies", or "ignore-requirements" - but only "replace" mode is used by here, so omitting for simplicity
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemdManagerEnableUnitReply {
    request: SystemdManagerEnableUnitRequest,
    changes: Vec<String>,
}

#[async_trait]
impl NatsRequestReplyHandler for SystemdManagerEnableUnitRequest {
    type Request = SystemdManagerEnableUnitRequest;
    type Reply = SystemdManagerEnableUnitReply;
    async fn handle(&self) -> Result<Self::Reply> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::systemd1::manager::ManagerProxy::new(&connection).await?;
        let (enablement_info, changes) = proxy.enable_unit_files(&self.files, false, false).await?;
        let reply = Self::Reply {
            changes,
            request: self.clone(),
        };
        Ok(reply)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitRequest(SystemdManagerRestartUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitRequest(SystemdManagerStartUnitRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitRequest(SystemdManagerStopUnitRequest),
    // #[serde(rename = "pi.command.systemctl")]
    // SystemctlCommandRequest(SystemctlCommandRequest),
    // #[serde(rename = "pi.printnanny_cloud.connect_account")]
    // ConnectCloudAccountRequest(ConnectCloudAccountRequest),
    // #[serde(rename = "pi.command.settings.gst_pipeline")]
    // GstPipelineSettingsRequest(SettingsRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsReply {
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.RestartUnit")]
    SystemdManagerRestartUnitReply(SystemdManagerRestartUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StartUnit")]
    SystemdManagerStartUnitReply(SystemdManagerStartUnitReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.StopUnit")]
    SystemdManagerStopUnitReply(SystemdManagerStartUnitReply),
    // #[serde(rename = "pi.command.settings.gst_pipeline")]
    // GstPipelineSettingsResponse(SettingsResponse),
    // #[serde(rename = "pi.command.connect_cloud_account")]
    // ConnectCloudAccountResponse(ConnectCloudAccountResponse),
}

#[async_trait]
impl NatsRequestReplyHandler for NatsRequest {
    type Request = NatsRequest;
    type Reply = NatsReply;

    async fn handle(&self) -> Result<NatsReply> {
        match self {
            NatsRequest::SystemdManagerStartUnitRequest(request) => Ok(
                NatsReply::SystemdManagerStartUnitReply(request.handle().await?),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test] // async test
    async fn test_dbus_systemd_manager_start_unit() {
        let request = SystemdManagerStartUnitRequest {
            name: "octoprint.service".into(),
        };
        let reply = request.handle().await.unwrap();
        assert_eq!(reply.request, request);
    }

    #[tokio::test] // async test
    async fn test_dbus_systemd_manager_restart_unit() {
        let request = SystemdManagerRestartUnitRequest {
            name: "octoprint.service".into(),
        };
        let reply = request.handle().await.unwrap();
        assert_eq!(reply.request, request);
    }

    #[tokio::test] // async test
    async fn test_dbus_systemd_manager_stop_unit() {
        let request = SystemdManagerStopUnitRequest {
            name: "octoprint.service".into(),
        };
        let reply = request.handle().await.unwrap();
        assert_eq!(reply.request, request);
    }

    #[tokio::test] // async test
    async fn test_dbus_systemd_manager_enable_init() {
        let request = SystemdManagerEnableUnitRequest {
            files: vec!["octoprint.service".into()],
        };
        let reply = request.handle().await.unwrap();
        assert_eq!(reply.request, request);
    }

    // fn test_gst_pipeline_settings_update_handler() {
    //     figment::Jail::expect_with(|jail| {
    //         let output = jail.directory().join("test.toml");

    //         jail.create_file(
    //             "test.toml",
    //             &format!(
    //                 r#"

    //             [tflite_model]
    //             tensor_width = 720
    //             "#,
    //             ),
    //         )?;
    //         jail.set_env("PRINTNANNY_GST_CONFIG", output.display());

    //         let src = "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4";

    //         let request_toml = r#"
    //             video_src = "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4"
    //             video_src_type = "Uri"
    //         "#;

    //         let request = SettingsRequest {
    //             data: request_toml.into(),
    //             format: SettingsFormat::Toml,
    //             subject: SettingsSubject::GstPipeline,
    //             pre_save: vec![],
    //             post_save: vec![],
    //         };

    //         let res = request.handle();

    //         assert_eq!(res.status, ResponseStatus::Ok);

    //         let saved_config = PrintNannyGstPipelineConfig::new().unwrap();
    //         assert_eq!(saved_config.video_src, src);
    //         assert_eq!(saved_config.video_src_type, VideoSrcType::Uri);
    //         Ok(())
    //     });
    // }

    // fn test_gst_octoprint_settings_update_handler() {
    //     figment::Jail::expect_with(|jail| {
    //         let output = jail.directory().join("test.toml");

    //         // configuration reference: https://docs.octoprint.org/en/master/configuration/config_yaml.html
    //         jail.create_file(
    //             "config.yaml",
    //             &format!(
    //                 r#"
    //             feature:
    //                 # Whether to enable the gcode viewer in the UI or not
    //                 gCodeVisualizer: true
    //             "#,
    //             ),
    //         )?;
    //         jail.set_env("OCTOPRINT_SETTINGS_FILE", output.display());

    //         let content = r#"
    //         feature:
    //             # Whether to enable the gcode viewer in the UI or not
    //             gCodeVisualizer: false
    //         "#;

    //         let request = SettingsRequest {
    //             data: content.into(),
    //             format: SettingsFormat::Yaml,
    //             subject: SettingsSubject::OctoPrint,
    //             pre_save: vec![],
    //             post_save: vec![],
    //         };

    //         let res = request.handle();

    //         assert_eq!(res.status, ResponseStatus::Ok);

    //         let saved_config = OctoPrintSettings::default().read_settings().unwrap();
    //         assert_eq!(saved_config, content);
    //         Ok(())
    //     });
    // }
}

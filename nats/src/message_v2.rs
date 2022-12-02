use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use printnanny_services::settings::printnanny::PrintNannySettings;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_asyncapi_models::{
    PrintNannyCloudAuthReply, PrintNannyCloudAuthRequest, SettingsApp, SettingsApplyReply,
    SettingsApplyRequest, SettingsFile, SettingsLoadReply, SettingsLoadRequest,
    SettingsRevertReply, SettingsRevertRequest, SystemdManagerDisableUnitsReply,
    SystemdManagerDisableUnitsRequest, SystemdManagerEnableUnitsReply,
    SystemdManagerEnableUnitsRequest, SystemdManagerGetUnitReply, SystemdManagerGetUnitRequest,
    SystemdManagerReloadUnitReply, SystemdManagerReloadUnitRequest, SystemdManagerRestartUnitReply,
    SystemdManagerRestartUnitRequest, SystemdManagerStartUnitReply, SystemdManagerStartUnitRequest,
    SystemdManagerStopUnitReply, SystemdManagerStopUnitRequest, SystemdUnitChange,
    SystemdUnitChangeState,
};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use printnanny_services::git2;
use printnanny_services::settings::vcs::VersionControlledSettings;

#[async_trait]
pub trait NatsRequestHandler {
    type Request: Serialize + DeserializeOwned + Clone + Debug + NatsRequestHandler;
    type Reply: Serialize + DeserializeOwned + Clone + Debug + NatsReplyBuilder;

    async fn handle(&self) -> Result<Self::Reply>;
}

#[async_trait]
pub trait NatsReplyBuilder {
    type Request: Serialize + DeserializeOwned + Clone + Debug + NatsRequestHandler;
    type Reply: Serialize + DeserializeOwned + Clone + Debug + NatsReplyBuilder;

    // async fn build_reply(&self, request: Self::Request) -> Result<Self::Reply>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsRequest {
    // pi.{pi}.settings.*
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthRequest(PrintNannyCloudAuthRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.load")]
    SettingsLoadRequest(SettingsLoadRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.apply")]
    SettingsApplyRequest(SettingsApplyRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.revert")]
    SettingsRevertRequest(SettingsRevertRequest),

    // pi.{pi}.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.{pi}.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitsRequest(SystemdManagerDisableUnitsRequest),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitsRequest(SystemdManagerEnableUnitsRequest),
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsReply {
    // pi.{pi}.settings.*
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.load")]
    SettingsLoadReply(SettingsLoadReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.apply")]
    SettingsApplyReply(SettingsApplyReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.revert")]
    SettingsRevertReply(SettingsRevertReply),

    // pi.{pi}.dbus.org.freedesktop.systemd1.*
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.DisableUnit")]
    SystemdManagerDisableUnitsReply(SystemdManagerDisableUnitsReply),
    #[serde(rename = "pi.dbus.org.freedesktop.systemd1.Manager.EnableUnit")]
    SystemdManagerEnableUnitsReply(SystemdManagerEnableUnitsReply),
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
}

#[async_trait]
impl NatsReplyBuilder for NatsReply {
    type Request = NatsRequest;
    type Reply = NatsReply;

    // async fn build_reply(&self, request: Self::Request) -> Result<Self::Reply> {}
}

impl NatsRequest {
    // handle messages sent to: "pi.{pi}.settings.printnanny.cloud.auth"
    pub async fn handle_printnanny_cloud_auth(
        &self,
        request: &PrintNannyCloudAuthRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let result = settings
            .connect_cloud_account(request.api_url.clone(), request.api_token.clone())
            .await;
        let result = match result {
            Ok(_) => NatsReply::PrintNannyCloudAuthReply(PrintNannyCloudAuthReply {
                status_code: 200,
                msg: format!("Success! Connected account: {}", request.email),
            }),
            Err(e) => NatsReply::PrintNannyCloudAuthReply(PrintNannyCloudAuthReply {
                status_code: 403,
                msg: format!("Error connecting account: {}", e.to_string()),
            }),
        };
        Ok(result)
    }

    pub async fn handle_printnanny_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;

        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.git_revert_hooks(Some(oid)).await?;
        let files = vec![settings.to_payload()?];
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
        let files = vec![settings.octoprint.to_payload()?];
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
        let files = vec![settings.moonraker.to_payload()?];
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
        let files = vec![settings.klipper.to_payload()?];
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

        for f in request.files.iter() {
            settings
                .save_and_commit(&f.content, Some(request.git_commit_msg.clone()))
                .await?;
        }
        let files = vec![settings.to_payload()?];
        self.build_settings_apply_reply(request, settings, files)
    }

    async fn handle_octoprint_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        for f in request.files.iter() {
            settings
                .octoprint
                .save_and_commit(&f.content, Some(request.git_commit_msg.clone()))
                .await?;
        }
        let files = vec![settings.octoprint.to_payload()?];
        self.build_settings_apply_reply(request, settings, files)
    }

    async fn handle_moonraker_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        for f in request.files.iter() {
            settings
                .moonraker
                .save_and_commit(&f.content, Some(request.git_commit_msg.clone()))
                .await?;
        }
        let files = vec![settings.moonraker.to_payload()?];
        self.build_settings_apply_reply(request, settings, files)
    }

    async fn handle_klipper_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        for f in request.files.iter() {
            settings
                .klipper
                .save_and_commit(&f.content, Some(request.git_commit_msg.clone()))
                .await?;
        }
        let files = vec![settings.klipper.to_payload()?];
        self.build_settings_apply_reply(request, settings, files)
    }

    fn build_settings_apply_reply(
        &self,
        request: &SettingsApplyRequest,
        settings: PrintNannySettings,
        files: Vec<SettingsFile>,
    ) -> Result<NatsReply> {
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        Ok(NatsReply::SettingsApplyReply(SettingsApplyReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        }))
    }

    fn handle_printnanny_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.to_payload()?];
        self.build_settings_load_reply(request, settings, files)
    }

    fn handle_octoprint_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.octoprint.to_payload()?];
        self.build_settings_load_reply(request, settings, files)
    }

    fn handle_moonraker_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.moonraker.to_payload()?];
        self.build_settings_load_reply(request, settings, files)
    }

    fn handle_klipper_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let files = vec![settings.klipper.to_payload()?];
        self.build_settings_load_reply(request, settings, files)
    }

    fn build_settings_load_reply(
        &self,
        request: &SettingsLoadRequest,
        settings: PrintNannySettings,
        files: Vec<SettingsFile>,
    ) -> Result<NatsReply> {
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let reply = SettingsLoadReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        };
        Ok(NatsReply::SettingsLoadReply(reply))
    }

    pub fn handle_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_load(request),
            SettingsApp::Octoprint => self.handle_octoprint_settings_load(request),
            SettingsApp::Moonraker => self.handle_moonraker_settings_load(request),
            SettingsApp::Klipper => self.handle_klipper_settings_load(request),
            _ => todo!(),
        }
    }

    pub async fn handle_settings_apply(&self, request: &SettingsApplyRequest) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_apply(request).await,
            SettingsApp::Octoprint => self.handle_octoprint_settings_apply(request).await,
            SettingsApp::Moonraker => self.handle_moonraker_settings_apply(request).await,
            SettingsApp::Klipper => self.handle_klipper_settings_apply(request).await,
            _ => todo!(),
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
            _ => todo!(),
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
        Ok(NatsReply::SystemdManagerEnableUnitsReply(
            SystemdManagerEnableUnitsReply { changes },
        ))
    }
}

#[async_trait]
impl NatsRequestHandler for NatsRequest {
    type Request = NatsRequest;
    type Reply = NatsReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let reply = match self {
            // pi.{pi}.settings.*
            NatsRequest::PrintNannyCloudAuthRequest(request) => {
                self.handle_printnanny_cloud_auth(request).await?
            }
            NatsRequest::SettingsLoadRequest(request) => self.handle_settings_load(request)?,
            NatsRequest::SettingsApplyRequest(request) => {
                self.handle_settings_apply(request).await?
            }
            NatsRequest::SettingsRevertRequest(request) => {
                self.handle_settings_revert(request).await?
            }
            // pi.{pi}.dbus.org.freedesktop.systemd1.*
            NatsRequest::SystemdManagerDisableUnitsRequest(request) => {
                self.handle_disable_units_request(request).await?
            }
            NatsRequest::SystemdManagerEnableUnitsRequest(request) => {
                self.handle_enable_units_request(request).await?
            }
            _ => todo!(),
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
            .block_on(settings.init_local_git_repo())
            .unwrap();
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
            let original = settings.to_payload().unwrap();
            let mut modified = original.clone();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            settings.paths.log_dir = "/path/to/testing".into();
            modified.content = settings.to_toml_string().unwrap();
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                files: vec![modified.clone()],
                app: Box::new(SettingsApp::Printnanny),
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
                assert_eq!(reply.files[0].content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest(SettingsLoadRequest {
                app: Box::new(SettingsApp::Printnanny),
            });
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

    #[cfg(feature = "systemd")]
    #[test]
    fn test_octoprint_settings_apply_load_revert() {
        figment::Jail::expect_with(|jail| {
            // init git repo in jail tmp dir
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();

            // apply a settings change
            let original = settings.octoprint.to_payload().unwrap();
            let mut modified = original.clone();
            modified.content = OCTOPRINT_MODIFIED_SETTINGS.into();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                files: vec![modified.clone()],
                app: Box::new(SettingsApp::Octoprint),
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
                assert_eq!(reply.files[0].content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest(SettingsLoadRequest {
                app: Box::new(SettingsApp::Octoprint),
            });
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

    #[cfg(feature = "systemd")]
    #[test]
    fn test_moonraker_settings_apply_load_revert() {
        figment::Jail::expect_with(|jail| {
            // init git repo in jail tmp dir
            make_settings_repo(jail);

            let settings = PrintNannySettings::new().unwrap();

            // apply a settings change
            let original = settings.moonraker.to_payload().unwrap();
            let mut modified = original.clone();
            modified.content = MOONRAKER_MODIFIED_SETTINGS.into();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::SettingsApplyRequest(SettingsApplyRequest {
                files: vec![modified.clone()],
                app: Box::new(SettingsApp::Moonraker),
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
                assert_eq!(reply.files[0].content, modified.content);
            } else {
                panic!("Expected NatsReply::SettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::SettingsLoadRequest(SettingsLoadRequest {
                app: Box::new(SettingsApp::Octoprint),
            });
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
}

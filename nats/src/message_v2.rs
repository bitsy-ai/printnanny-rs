use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use printnanny_services::settings::printnanny::PrintNannySettings;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_asyncapi_models::{
    PrintNannyCloudAuthReply, PrintNannyCloudAuthRequest, SettingsApp, SettingsApplyReply,
    SettingsApplyRequest, SettingsFile, SettingsLoadReply, SettingsLoadRequest,
    SettingsRevertReply, SettingsRevertRequest, SystemdManagerGetUnitReply,
    SystemdManagerGetUnitRequest,
};

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
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthRequest(PrintNannyCloudAuthRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.load")]
    SettingsLoadRequest(SettingsLoadRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.apply")]
    SettingsApplyRequest(SettingsApplyRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.revert")]
    SettingsRevertRequest(SettingsRevertRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsReply {
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.load")]
    SettingsLoadReply(SettingsLoadReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.apply")]
    SettingsApplyReply(SettingsApplyReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.revert")]
    SettingsRevertReply(SettingsRevertReply),
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

    // handle messages sent to: "pi.settings.printnanny.revert"
    pub async fn handle_printnanny_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;

        // revert commit
        let oid = git2::Oid::from_str(&request.git_commit)?;
        settings.git_revert(Some(oid))?;

        // build response
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let files = vec![settings.to_payload()?];
        Ok(NatsReply::SettingsRevertReply(SettingsRevertReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        }))
    }

    pub async fn handle_printnanny_settings_apply(
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

    pub async fn handle_octoprint_settings_apply(
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

    fn build_settings_apply_reply(
        &self,
        request: &SettingsApplyReply,
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
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let files = vec![settings.to_payload()?];

        let reply = SettingsLoadReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        };
        Ok(NatsReply::SettingsLoadReply(reply))
    }

    fn handle_octoprint_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        let git_head_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let files = vec![settings.octoprint.to_payload()?];
        let reply = SettingsLoadReply {
            app: request.app.clone(),
            files,
            git_head_commit,
            git_history,
        };
        Ok(NatsReply::SettingsLoadReply(reply))
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
            SettingsApp::Klipper => self.handle_moonraker_settings_load(request).await,
            _ => todo!(),
        }
    }

    pub async fn handle_settings_revert(
        &self,
        request: &SettingsRevertRequest,
    ) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_revert(request).await,
            _ => todo!(),
        }
    }
}

#[async_trait]
impl NatsRequestHandler for NatsRequest {
    type Request = NatsRequest;
    type Reply = NatsReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let reply = match self {
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
            _ => todo!(),
        };

        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printnanny_asyncapi_models::SettingsFormat;
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
            Ok(())
        });
    }
}

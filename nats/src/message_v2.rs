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
    PrintNannySettingsLoadRequest(SettingsLoadRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.apply")]
    PrintNannySettingsApplyRequest(SettingsApplyRequest),
    #[serde(rename = "pi.{pi}.settings.vcs.revert")]
    PrintNannySettingsRevertRequest(SettingsRevertRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsReply {
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.load")]
    PrintNannySettingsLoadReply(SettingsLoadReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.apply")]
    PrintNannySettingsApplyReply(SettingsApplyReply),
    #[serde(rename = "pi.{pi}.settings.printnanny.revert")]
    PrintNannySettingsRevertReply(SettingsRevertReply),
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

    // pub async fn handle_klipper_settings_load(&self, request: SettingsLoadRequest) -> Result<NatsReply> {
    //     let settings = PrintNannySettings::new()?;

    //     let git_head_commit = settings.get_git_head_commit()?.oid;
    //     let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
    //         settings.get_rev_list()?.iter().map(|r| r.into()).collect();

    //     let content =

    // }

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
        Ok(NatsReply::PrintNannySettingsRevertReply(
            SettingsRevertReply {
                app: request.app.clone(),
                files,
                git_head_commit,
                git_history,
            },
        ))
    }

    // handle messages sent to "pi.settings.printnanny.apply")
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
        let git_head_commit = settings.get_git_head_commit()?.oid;

        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let files = vec![settings.to_payload()?];

        Ok(NatsReply::PrintNannySettingsApplyReply(
            SettingsApplyReply {
                app: request.app.clone(),
                files,
                git_head_commit,
                git_history,
            },
        ))
    }

    pub fn handle_printnanny_settings_load(
        &self,
        request: &SettingsLoadRequest,
    ) -> Result<NatsReply> {
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
        Ok(NatsReply::PrintNannySettingsLoadReply(reply))
    }

    pub fn handle_settings_load(&self, request: &SettingsLoadRequest) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_load(request),
            _ => todo!(),
        }
    }

    pub async fn handle_settings_apply(&self, request: &SettingsApplyRequest) -> Result<NatsReply> {
        match *request.app {
            SettingsApp::Printnanny => self.handle_printnanny_settings_apply(request).await,
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
            NatsRequest::PrintNannySettingsLoadRequest(request) => {
                self.handle_settings_load(request)?
            }
            NatsRequest::PrintNannySettingsApplyRequest(request) => {
                self.handle_settings_apply(request).await?
            }
            NatsRequest::PrintNannySettingsRevertRequest(request) => {
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
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            settings.paths.log_dir = "/path/to/testing".into();
            let git_commit_msg = "testing".to_string();

            let request_apply = NatsRequest::PrintNannySettingsApplyRequest(SettingsApplyRequest {
                files: vec![original.clone()],
                app: Box::new(SettingsApp::Printnanny),
                git_head_commit,
                git_commit_msg: git_commit_msg.clone(),
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_apply.handle())
                .unwrap();
            let revert_commit = settings.get_git_head_commit().unwrap().oid;

            if let NatsReply::PrintNannySettingsApplyReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
            } else {
                panic!("Expected NatsReply::PrintNannySettingsApplyReply")
            }

            // load the settings we just applied
            let request_load = NatsRequest::PrintNannySettingsLoadRequest(SettingsLoadRequest {
                app: Box::new(SettingsApp::Printnanny),
            });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_load.handle())
                .unwrap();
            let reply = if let NatsReply::PrintNannySettingsLoadReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
                assert_eq!(reply.git_head_commit, revert_commit);
                reply
            } else {
                panic!("Expected NatsReply::PrintNannySettingsLoadReply")
            };

            // revert the settings
            let request_revert =
                NatsRequest::PrintNannySettingsRevertRequest(SettingsRevertRequest {
                    git_commit: revert_commit,
                    app: Box::new(SettingsApp::Printnanny),
                    files: reply.files,
                });
            let reply = Runtime::new()
                .unwrap()
                .block_on(request_revert.handle())
                .unwrap();
            if let NatsReply::PrintNannySettingsRevertReply(reply) = reply {
                assert_eq!(reply.files[0].content, original.content);
            } else {
                panic!("Expected NatsReply::PrintNannySettingsRevertReply")
            }

            Ok(())
        })
    }

    // #[test]
    // fn test_load_klipper_settings() {
    //     let request = NatsRequest::KlipperSettingsLoadRequest(SettingsLoadRequest {
    //         format: Box::new(printnanny_asyncapi_models::SettingsFormat::Toml),
    //         filename: Box::new(printnanny_asyncapi_models::SettingsFile::PrintnannyDotToml),
    //     });
    //     figment::Jail::expect_with(|jail| {
    //         make_settings_repo(jail);
    //         let reply = Runtime::new().unwrap().block_on(request.handle()).unwrap();
    //         if let NatsReply::PrintNannySettingsLoadReply(reply) = reply {
    //             let settings = PrintNannySettings::new().unwrap();
    //             let expected = settings.to_toml_string().unwrap();
    //             assert_eq!(reply.content, expected);
    //         } else {
    //             panic!("Expected NatsReply::PrintNannySettingsLoadReply")
    //         }
    //         Ok(())
    //     });
    // }
}

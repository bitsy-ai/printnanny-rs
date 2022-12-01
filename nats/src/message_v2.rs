use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use printnanny_services::settings::printnanny::PrintNannySettings;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_asyncapi_models::{
    PrintNannyCloudAuthReply, PrintNannyCloudAuthRequest, SettingsApplyReply, SettingsApplyRequest,
    SettingsFile, SettingsFormat, SettingsLoadReply, SettingsLoadRequest, SettingsRevertReply,
    SettingsRevertRequest, SystemdManagerGetUnitReply, SystemdManagerGetUnitRequest,
};

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
    #[serde(rename = "pi.settings.printnanny.load")]
    PrintNannySettingsLoadRequest(SettingsLoadRequest),
    #[serde(rename = "pi.settings.printnanny.apply")]
    PrintNannySettingsApplyRequest(SettingsApplyRequest),
    // #[serde(rename = "pi.settings.printnanny.revert")]
    // PrintNannySettingsRevertRequest(SettingsRevertRequest),
    // #[serde(rename = "pi.settings.klipper.load")]
    // KlipperSettingsLoadRequest(SettingsLoadRequest),
    // #[serde(rename = "pi.settings.klipper.apply")]
    // KlipperSettingsApplyRequest(SettingsApplyRequest),
    // #[serde(rename = "pi.settings.klipper.revert")]
    // KlipperSettingsRevertRequest(SettingsRevertRequest),

    // #[serde(rename = "pi.settings.moonraker.load")]
    // MoonrakerSettingsLoadRequest(SettingsLoadRequest),
    // #[serde(rename = "pi.settings.moonraker.apply")]
    // MoonrakerSettingsApplyRequest(SettingsApplyRequest),
    // #[serde(rename = "pi.settings.moonraker.revert")]
    // MoonrakerSettingsRevertRequest(SettingsRevertRequest),

    // #[serde(rename = "pi.settings.octoprint.load")]
    // OctoPrintSettingsLoadRequest(SettingsLoadRequest),
    // #[serde(rename = "pi.settings.octoprint.apply")]
    // OctoPrintSettingsApplyRequest(SettingsApplyRequest),
    // #[serde(rename = "pi.settings.octoprint.revert")]
    // OctoPrintSettingsRevertRequest(SettingsRevertRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsReply {
    #[serde(rename = "pi.{pi_id}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.settings.printnanny.load")]
    PrintNannySettingsLoadReply(SettingsLoadReply),
    #[serde(rename = "pi.settings.gst_pipeline.apply")]
    PrintNannySettingsApplyReply(SettingsApplyReply),
    // #[serde(rename = "pi.settings.gst_pipeline.revert")]
    // PrintNannySettingsRevertReply(SettingsRevertReply),
    // #[serde(rename = "pi.settings.klipper.load")]
    // KlipperSettingsLoadReply(SettingsLoadReply),
    // #[serde(rename = "pi.settings.klipper.apply")]
    // KlipperSettingsApplyReply(SettingsApplyReply),
    // #[serde(rename = "pi.settings.klipper.revert")]
    // KlipperSettingsRevertReply(SettingsRevertReply),

    // #[serde(rename = "pi.settings.moonraker.load")]
    // MoonrakerSettingsLoadReply(SettingsLoadReply),
    // #[serde(rename = "pi.settings.moonraker.apply")]
    // MoonrakerSettingsApplyReply(SettingsApplyReply),
    // #[serde(rename = "pi.settings.moonraker.revert")]
    // MoonrakerSettingsRevertReply(SettingsRevertReply),

    // #[serde(rename = "pi.settings.octoprint.load")]
    // OctoPrintSettingsLoadReply(SettingsLoadReply),
    // #[serde(rename = "pi.settings.octoprint.apply")]
    // OctoPrintSettingsApplyReply(SettingsApplyReply),
    // #[serde(rename = "pi.settings.octoprint.revert")]
    // OctoPrintSettingsRevertReply(SettingsRevertReply),
}

#[async_trait]
impl NatsReplyBuilder for NatsReply {
    type Request = NatsRequest;
    type Reply = NatsReply;

    // async fn build_reply(&self, request: Self::Request) -> Result<Self::Reply> {}
}

impl NatsRequest {
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

    pub async fn handle_printnanny_settings_apply(
        &self,
        request: &SettingsApplyRequest,
    ) -> Result<NatsReply> {
        let settings = PrintNannySettings::new()?;
        settings
            .save_and_commit(&request.content, Some(request.git_commit_msg.clone()))
            .await?;
        let settings = PrintNannySettings::new()?;
        let content = settings.to_toml_string()?;
        let git_head_commit = settings.get_git_head_commit()?.oid;

        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        Ok(NatsReply::PrintNannySettingsApplyReply(
            SettingsApplyReply {
                format: request.format.clone(),
                filename: request.filename.clone(),
                git_head_commit,
                git_history,
                content,
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
        let reply = SettingsLoadReply {
            format: Box::new(SettingsFormat::Toml),
            filename: Box::new(SettingsFile::PrintnannyDotToml),
            content: settings.to_toml_string()?,
            git_head_commit,
            git_history,
        };
        Ok(NatsReply::PrintNannySettingsLoadReply(reply))
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
                self.handle_printnanny_settings_load(request)?
            }
            NatsRequest::PrintNannySettingsApplyRequest(request) => {
                self.handle_printnanny_settings_apply(request).await?
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
    fn test_load_printnanny_settings() {
        let request = NatsRequest::PrintNannySettingsLoadRequest(SettingsLoadRequest {
            format: Box::new(printnanny_asyncapi_models::SettingsFormat::Toml),
            filename: Box::new(printnanny_asyncapi_models::SettingsFile::PrintnannyDotToml),
        });
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);
            let reply = Runtime::new().unwrap().block_on(request.handle()).unwrap();
            if let NatsReply::PrintNannySettingsLoadReply(reply) = reply {
                let settings = PrintNannySettings::new().unwrap();
                let expected = settings.to_toml_string().unwrap();
                assert_eq!(reply.content, expected);
            } else {
                panic!("Expected NatsReply::PrintNannySettingsLoadReply")
            }
            Ok(())
        });
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
    fn test_printnanny_settings_apply() {
        figment::Jail::expect_with(|jail| {
            make_settings_repo(jail);
            let mut settings = PrintNannySettings::new().unwrap();
            settings.paths.log_dir = "/path/to/testing".into();

            let content = settings.to_toml_string().unwrap();
            let git_head_commit = settings.get_git_head_commit().unwrap().oid;
            let git_commit_msg = "testing".to_string();

            let request = NatsRequest::PrintNannySettingsApplyRequest(SettingsApplyRequest {
                format: Box::new(SettingsFormat::Toml),
                filename: Box::new(SettingsFile::PrintnannyDotToml),
                content,
                git_head_commit,
                git_commit_msg: git_commit_msg.clone(),
            });

            let reply = Runtime::new().unwrap().block_on(request.handle()).unwrap();
            if let NatsReply::PrintNannySettingsApplyReply(reply) = reply {
                assert_eq!(reply.git_history[0].message, git_commit_msg);
            } else {
                panic!("Expected NatsReply::PrintNannyCloudAuthReply")
            }
            Ok(())
        })
    }
}

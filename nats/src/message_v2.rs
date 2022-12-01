use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use printnanny_services::settings::PrintNannySettings;
use printnanny_services::vcs::VersionControlledSettings;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_asyncapi_models::{
    PrintNannyCloudAuthReply, PrintNannyCloudAuthRequest, SettingsApplyReply, SettingsApplyRequest,
    SettingsFile, SettingsFormat, SettingsLoadReply, SettingsLoadRequest, SettingsRevertReply,
    SettingsRevertRequest, SystemdManagerGetUnitReply, SystemdManagerGetUnitRequest,
};

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
pub enum SettingsRequest {
    #[serde(rename = "pi.{pi}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthRequest(PrintNannyCloudAuthRequest),
    #[serde(rename = "pi.settings.printnanny.load")]
    PrintNannySettingsLoadRequest(SettingsLoadRequest),
    // #[serde(rename = "pi.settings.printnanny.apply")]
    // PrintNannySettingsApplyRequest(SettingsApplyRequest),
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
pub enum SettingsReply {
    #[serde(rename = "pi.{pi_id}.settings.printnanny.cloud.auth")]
    PrintNannyCloudAuthReply(PrintNannyCloudAuthReply),
    #[serde(rename = "pi.settings.printnanny.load")]
    PrintNannySettingsLoadReply(SettingsLoadReply),
    // #[serde(rename = "pi.settings.gst_pipeline.apply")]
    // PrintNannySettingsApplyReply(SettingsApplyReply),
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
impl NatsReplyBuilder for SettingsReply {
    type Request = SettingsRequest;
    type Reply = SettingsReply;

    // async fn build_reply(&self, request: Self::Request) -> Result<Self::Reply> {}
}

impl SettingsRequest {
    pub fn handle_printnanny_settings_load(
        &self,
        request: &SettingsLoadRequest,
    ) -> Result<SettingsReply> {
        let settings = PrintNannySettings::new()?;
        let head_git_commit = settings.get_git_head_commit()?.oid;
        let git_history: Vec<printnanny_asyncapi_models::GitCommit> =
            settings.get_rev_list()?.iter().map(|r| r.into()).collect();
        let reply = SettingsLoadReply {
            format: Box::new(SettingsFormat::Toml),
            filename: Box::new(SettingsFile::PrintnannyDotToml),
            content: settings.to_toml_string()?,
            head_git_commit,
            git_history,
        };
        Ok(SettingsReply::PrintNannySettingsLoadReply(reply))
    }
}

#[async_trait]
impl NatsRequestHandler for SettingsRequest {
    type Request = SettingsRequest;
    type Reply = SettingsReply;

    async fn handle(&self) -> Result<Self::Reply> {
        let reply = match self {
            SettingsRequest::PrintNannySettingsLoadRequest(request) => {
                self.handle_printnanny_settings_load(request)?
            }
            _ => todo!(),
        };

        Ok(reply)
    }
}

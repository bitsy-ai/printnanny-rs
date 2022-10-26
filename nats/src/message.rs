use std::collections::HashMap;
use std::fmt::Debug;
use std::process;

use anyhow::Result;
use async_process::Output;
use futures::executor;
use log::{error, info};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_services::config::PrintNannyConfig;
use printnanny_services::figment;
use printnanny_services::figment::providers::Format;
use printnanny_services::printnanny_api::ApiService;

use printnanny_gst_config::config::PrintNannyGstPipelineConfig;
use printnanny_services::systemd::{systemctl_list_enabled_units, systemctl_show_payload};

pub trait MessageHandler<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug,
    Response: Serialize + DeserializeOwned + Debug,
{
    fn handle(&self, request: &Request) -> Result<Response>;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SystemctlCommand {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "restart")]
    Restart,
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "enable")]
    Enable,
    #[serde(rename = "disable")]
    Disable,
    #[serde(rename = "list_enabled")]
    ListEnabled,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MediaCommand {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResponseStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandRequest {
    service: String,
    command: SystemctlCommand,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectCloudAccountRequest {
    email: String,
    api_token: String,
    api_uri: String,
}

impl ConnectCloudAccountRequest {
    async fn _handle(&self) -> Result<ConnectCloudAccountResponse> {
        let mut config = PrintNannyConfig::new()?;
        config.cloud.api.base_path = self.api_uri.clone();
        config.cloud.api.bearer_access_token = Some(self.api_token.clone());
        config.try_save()?;

        let mut api_service = ApiService::new(config)?;
        api_service.sync().await?;

        let res = ConnectCloudAccountResponse {
            request: Some(self.clone()),
            detail: format!(
                "Success! Connected PrintNanny Cloud account belonging to {}",
                self.email
            ),
            status: ResponseStatus::Ok,
        };
        Ok(res)
    }

    fn handle(&self) -> ConnectCloudAccountResponse {
        match executor::block_on(self._handle()) {
            Ok(r) => r,
            Err(e) => {
                let detail = format!("Error linking PrintNanny Cloud account: {:?}", e);
                error!("{}", &detail);
                ConnectCloudAccountResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineConfigRequest {
    json: String, // json string, intended for use with Figment.rs JSON provider: https://docs.rs/figment/latest/figment/providers/struct.Json.html
    pre_save: Vec<SystemctlCommandRequest>, // run commands prior to applying config merge/save
    post_save: Vec<SystemctlCommandRequest>, // run commands after applying config merge/save
}

impl GstPipelineConfigRequest {
    // merge incoming "figment" (configurationf fragment) with existing configuration, sourced from .json/.toml serializable data structure and env variables prefixed with PRINTNANNY_
    fn _handle(&self) -> Result<(Vec<SystemctlCommandResponse>, Vec<SystemctlCommandResponse>)> {
        // build a config fragment from json string
        let incoming = figment::providers::Json::string(&self.json);
        let figment = PrintNannyGstPipelineConfig::figment()?.merge(incoming);
        let config: PrintNannyGstPipelineConfig = figment.extract()?;

        // run pre-save command hooks
        info!("Running pre-save commands: {:?}", self.pre_save);
        let pre_save: Vec<SystemctlCommandResponse> = self
            .pre_save
            .iter()
            .map(|request| request.handle())
            .collect();
        info!("Finished running pre-save commands: {:?}", pre_save);
        // save merged configuration
        config.try_save().expect("Failed to save configuration");

        // run post-save command hooks
        info!("Running pre-save commands: {:?}", self.pre_save);
        let post_save: Vec<SystemctlCommandResponse> = self
            .post_save
            .iter()
            .map(|request| request.handle())
            .collect();
        info!("Finished running post-save commands {:?}", post_save);

        Ok((pre_save, post_save))
    }

    pub fn handle(&self) -> GstPipelineConfigResponse {
        match self._handle() {
            Ok((pre_save, post_save)) => GstPipelineConfigResponse {
                pre_save,
                post_save,
                request: Some(self.clone()),
                detail: "Updated PrintNanny configuration".into(),
                status: ResponseStatus::Ok,
            },
            Err(e) => {
                let detail = format!("Error updating PrintNanny configuration: {:?}", e);
                error!("{}", &detail);
                GstPipelineConfigResponse {
                    pre_save: vec![],
                    post_save: vec![],
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail,
                }
            }
        }
    }
}

impl SystemctlCommandRequest {
    fn build_response(&self, output: &Output) -> Result<SystemctlCommandResponse> {
        let data: HashMap<String, serde_json::Value> = HashMap::new();
        let res = match output.status.success() {
            true => {
                let detail = String::from_utf8(output.stdout.clone())?;
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Ok,
                    detail,
                    data,
                }
            }
            false => {
                let detail = String::from_utf8(output.stderr.clone())?;
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail,
                    data,
                }
            }
        };
        Ok(res)
    }

    fn handle(&self) -> SystemctlCommandResponse {
        let result = match self.command {
            SystemctlCommand::ListEnabled => self.list_enabled(),
            SystemctlCommand::Start => self._systemctl_action("start"),
            SystemctlCommand::Stop => self._systemctl_action("stop"),
            SystemctlCommand::Restart => self._systemctl_action("start"),
            SystemctlCommand::Status => self.status(),
            SystemctlCommand::Enable => self._systemctl_action("enable"),
            SystemctlCommand::Disable => self._systemctl_action("disable"),
        };
        match result {
            Ok(response) => response,
            Err(e) => {
                let data: HashMap<String, serde_json::Value> = HashMap::new();
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail: format!("Error running {:?} {}: {:?}", self.command, self.service, e),
                    data,
                }
            }
        }
    }

    fn _systemctl_action(&self, action: &str) -> Result<SystemctlCommandResponse> {
        let args = ["systemctl", action, &self.service];
        let output = process::Command::new("sudo").args(&args).output()?;
        info!("{:?} stdout: {:?}", args, output.stdout);
        if !output.stdout.is_empty() {
            error!("{:?} stdout: {:?}", args, output.stdout);
        }
        self.build_response(&output)
    }

    fn list_enabled(&self) -> Result<SystemctlCommandResponse> {
        let (output, unitmap) = systemctl_list_enabled_units()?;
        let mut res = self.build_response(&output)?;
        for (key, value) in unitmap.iter() {
            res.data.insert(key.clone(), serde_json::to_value(value)?);
        }
        Ok(res)
    }

    fn status(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("systemctl")
            .args(&["show", &self.service])
            .output()?;

        let mut res = self.build_response(&output)?;
        res.data = systemctl_show_payload(res.detail.as_bytes())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandResponse {
    request: Option<SystemctlCommandRequest>,
    status: ResponseStatus,
    detail: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GstPipelineConfigResponse {
    request: Option<GstPipelineConfigRequest>,
    status: ResponseStatus,
    detail: String,
    pre_save: Vec<SystemctlCommandResponse>,
    post_save: Vec<SystemctlCommandResponse>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectCloudAccountResponse {
    request: Option<ConnectCloudAccountRequest>,
    status: ResponseStatus,
    detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandRequest(SystemctlCommandRequest),
    #[serde(rename = "pi.command.gst_pipeline_config")]
    GstPipelineConfigRequest(GstPipelineConfigRequest),
    #[serde(rename = "pi.command.connect_cloud_account")]
    ConnectCloudAccountRequest(ConnectCloudAccountRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsResponse {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandResponse(SystemctlCommandResponse),
    #[serde(rename = "pi.command.gst_pipeline_config")]
    GstPipelineConfigResponse(GstPipelineConfigResponse),
    #[serde(rename = "pi.command.connect_cloud_account")]
    ConnectCloudAccountResponse(ConnectCloudAccountResponse),
}

impl NatsResponse {}

impl MessageHandler<NatsRequest, NatsResponse> for NatsRequest {
    fn handle(&self, request: &NatsRequest) -> Result<NatsResponse> {
        match request {
            NatsRequest::SystemctlCommandRequest(request) => {
                Ok(NatsResponse::SystemctlCommandResponse(request.handle()))
            }
            NatsRequest::GstPipelineConfigRequest(request) => {
                Ok(NatsResponse::GstPipelineConfigResponse(request.handle()))
            }
            NatsRequest::ConnectCloudAccountRequest(request) => {
                Ok(NatsResponse::ConnectCloudAccountResponse(request.handle()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printnanny_gst_config::config::VideoSrcType;
    use printnanny_services::systemd;

    #[test]
    fn test_gst_pipeline_config_update_handler() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().join("test.toml");

            jail.create_file(
                "test.toml",
                &format!(
                    r#"

                [tflite_model]
                tensor_width = 720
                "#,
                ),
            )?;
            jail.set_env("PRINTNANNY_GST_CONFIG", output.display());

            let src = "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4";

            let request_json = r#"{ "video_src": "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4", "video_src_type": "Uri"}"#;

            let request = GstPipelineConfigRequest {
                json: request_json.into(),
                pre_save: vec![],
                post_save: vec![],
            };

            let res = request.handle();

            assert_eq!(res.status, ResponseStatus::Ok);

            let saved_config = PrintNannyGstPipelineConfig::new().unwrap();
            assert_eq!(saved_config.video_src, src);
            assert_eq!(saved_config.video_src_type, VideoSrcType::Uri);
            Ok(())
        });
    }

    #[test]
    fn test_systemctl_list_units() {
        let request = SystemctlCommandRequest {
            service: "".into(),
            command: SystemctlCommand::ListEnabled,
        };

        let res = request.list_enabled().unwrap();

        let (_, unit) = res.data.iter().next().unwrap();

        let unit = serde_json::from_value::<systemd::SystemctlListUnit>(unit.clone()).unwrap();
        assert_eq!(unit.state, "enabled");
    }
}

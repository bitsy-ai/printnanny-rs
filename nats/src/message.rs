use std::collections::HashMap;
use std::fmt::Debug;
use std::process;

use anyhow::Result;
use async_process::Output;
use log::info;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_services::config::PrintNannyConfig;
use printnanny_services::figment;
use printnanny_services::figment::providers::Format;

use crate::util::{self, SystemctlListUnit};

pub trait MessageHandler<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug,
    Response: Serialize + DeserializeOwned + Debug,
{
    fn handle(&self, request: &Request) -> Result<Response>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaCommand {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandRequest {
    service: String,
    command: SystemctlCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiConfigRequest {
    json: String, // json string, intended for use with Figment.rs JSON provider: https://docs.rs/figment/latest/figment/providers/struct.Json.html
    pre_save: Vec<SystemctlCommandRequest>, // run commands prior to applying config merge/save
    post_save: Vec<SystemctlCommandRequest>, // run commands after applying config merge/save
}

impl PiConfigRequest {
    // merge incoming "figment" (configurationf fragment) with existing configuration, sourced from .json/.toml serializable data structure and env variables prefixed with PRINTNANNY_
    fn _handle(&self) -> Result<(Vec<SystemctlCommandResponse>, Vec<SystemctlCommandResponse>)> {
        // build a config fragment from json string
        let incoming = figment::providers::Json::string(&self.json);
        let figment = PrintNannyConfig::figment()?.merge(incoming);
        let config: PrintNannyConfig = figment.extract()?;

        // run pre-save command hooks
        info!("Running pre-save commands: {:?}", self.pre_save);
        let pre_save: Vec<SystemctlCommandResponse> = self
            .pre_save
            .iter()
            .map(|request| request.handle())
            .collect();
        info!("Finished running post-save commands, attempting to save merged configuration");
        // save merged configuration
        config.try_save()?;

        // run post-save command hooks
        info!("Running pre-save commands: {:?}", self.pre_save);
        let post_save: Vec<SystemctlCommandResponse> = self
            .post_save
            .iter()
            .map(|request| request.handle())
            .collect();
        info!("Finished running post-save commands, attempting to save merged configuration");

        Ok((pre_save, post_save))
    }

    pub fn handle(&self) -> PiConfigResponse {
        match self._handle() {
            Ok((pre_save, post_save)) => PiConfigResponse {
                pre_save,
                post_save,
                request: Some(self.clone()),
                detail: "Updated PrintNanny configuration".into(),
                status: ResponseStatus::Ok,
            },
            Err(e) => PiConfigResponse {
                pre_save: vec![],
                post_save: vec![],
                request: Some(self.clone()),
                detail: format!("Error updating PrintNanny configuration: {:?}", e),
                status: ResponseStatus::Error,
            },
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
                    detail: detail,
                    data,
                }
            }
            false => {
                let detail = String::from_utf8(output.stderr.clone())?;
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail: detail,
                    data,
                }
            }
        };
        Ok(res)
    }

    fn handle(&self) -> SystemctlCommandResponse {
        let result = match self.command {
            SystemctlCommand::ListEnabled => self.list_enabled(),
            SystemctlCommand::Start => self.start(),
            SystemctlCommand::Stop => self.stop(),
            SystemctlCommand::Restart => self.restart(),
            SystemctlCommand::Status => self.status(),
            SystemctlCommand::Enable => self.enable(),
            SystemctlCommand::Disable => self.disable(),
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

    fn list_enabled(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("systemctl")
            .args(&["list-unit-files", "--state=enabled", "--output=json"])
            .output()?;
        let mut res = self.build_response(&output)?;
        let list_units = serde_json::from_slice::<Vec<SystemctlListUnit>>(&output.stdout)?;
        for unit in list_units.iter() {
            res.data
                .insert(unit.unit_file.clone(), serde_json::to_value(unit)?);
        }
        Ok(res)
    }

    fn disable(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "disable", &self.service])
            .output()?;
        self.build_response(&output)
    }

    fn enable(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "enable", &self.service])
            .output()?;
        self.build_response(&output)
    }
    fn start(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "start", &self.service])
            .output()?;
        self.build_response(&output)
    }
    fn stop(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "stop", &self.service])
            .output()?;
        self.build_response(&output)
    }
    fn restart(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "restart", &self.service])
            .output()?;
        self.build_response(&output)
    }
    fn status(&self) -> Result<SystemctlCommandResponse> {
        let output = process::Command::new("systemctl")
            .args(&["show", &self.service])
            .output()?;

        let mut res = self.build_response(&output)?;
        res.data = util::systemctl_show_payload(res.detail.as_bytes())?;
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandResponse {
    request: Option<SystemctlCommandRequest>,
    status: ResponseStatus,
    detail: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiConfigResponse {
    request: Option<PiConfigRequest>,
    status: ResponseStatus,
    detail: String,
    pre_save: Vec<SystemctlCommandResponse>,
    post_save: Vec<SystemctlCommandResponse>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandRequest(SystemctlCommandRequest),
    #[serde(rename = "pi.config")]
    PiConfigRequest(PiConfigRequest),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsResponse {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandResponse(SystemctlCommandResponse),
    #[serde(rename = "pi.config")]
    PiConfigResponse(PiConfigResponse),
}

impl NatsResponse {}

impl MessageHandler<NatsRequest, NatsResponse> for NatsRequest {
    fn handle(&self, request: &NatsRequest) -> Result<NatsResponse> {
        match request {
            NatsRequest::SystemctlCommandRequest(request) => {
                Ok(NatsResponse::SystemctlCommandResponse(request.handle()))
            }
            NatsRequest::PiConfigRequest(request) => {
                Ok(NatsResponse::PiConfigResponse(request.handle()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printnanny_services::config::VideoSrcType;
    use printnanny_services::paths::PRINTNANNY_CONFIG_FILENAME;

    #[test]
    fn test_pi_config_update_handler() {
        figment::Jail::expect_with(|jail| {
            let output = jail.directory().to_str().unwrap();

            jail.create_file(
                PRINTNANNY_CONFIG_FILENAME,
                &format!(
                    r#"
                profile = "default"

                [paths]
                etc = "{output}/etc"
                run = "{output}/run"
                log = "{output}/log"
                "#,
                    output = output
                ),
            )?;
            jail.set_env("PRINTNANNY_CONFIG", PRINTNANNY_CONFIG_FILENAME);

            let default_config = PrintNannyConfig::new().unwrap();
            default_config.paths.try_init_dirs().unwrap();

            let src = "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4";

            let request_json = r#"{
                "vision": { "src": "https://cdn.printnanny.ai/gst-demo-videos/demo_video_1.mp4", "video_src_type": "Uri"}
            }"#;

            let request = PiConfigRequest {
                json: request_json.into(),
                pre_save: vec![],
                post_save: vec![],
            };

            let res = request.handle();

            assert_eq!(res.status, ResponseStatus::Ok);

            let saved_config = PrintNannyConfig::new().unwrap();
            assert_eq!(saved_config.vision.video_src, src);
            assert_eq!(saved_config.vision.video_src_type, VideoSrcType::Uri);
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

        let unit = serde_json::from_value::<util::SystemctlListUnit>(unit.clone()).unwrap();
        assert_eq!(unit.state, "enabled");
    }
}

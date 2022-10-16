use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::process;
use std::{fmt, io::Write};

use anyhow::Result;
use async_process::Output;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::util::{self, SystemctlListUnit};

const DEFAULT_GST_STREAM_CONF: &str = "/var/run/printnanny/printnanny-vision.conf";

pub trait MessageHandler<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug,
    Response: Serialize + DeserializeOwned + Debug,
{
    fn handle(&self, request: &Request) -> Result<Response>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JanusMedia {
    age_ms: u64,
    codec: String,
    label: String,
    mid: String,
    mindex: i32,
    port: i32,
    pt: i32,
    rtpmap: String,
    #[serde(rename = "type")]
    _type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoStreamSource {
    File,
    Device,
}

impl fmt::Display for VideoStreamSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::File => write!(f, "{}", "file"),
            Self::Device => write!(f, "{}", "device"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JanusStreamMetadata {
    path: String,
    video_stream_src: VideoStreamSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JanusStream {
    description: String,
    enabled: bool,
    id: i32,
    media: Vec<JanusMedia>,
    metadata: JanusStreamMetadata,
    name: String,
    #[serde(rename = "type")]
    _type: String,
    viewers: i32,
}

impl JanusStream {
    pub fn gst_pipeline_conf(&self) -> String {
        let media = self.media.get(0).expect("Expected JanusMedia to be set");
        format!(
            r#"UDP_PORT={udp_port}
        INPUT_PATH={input_path}
        VIDEO_STREAM_SRC={video_stream_src}"#,
            udp_port = media.port,
            input_path = self.metadata.path,
            video_stream_src = self.metadata.video_stream_src,
        )
    }
    pub fn write_gst_pipeline_conf(&self) -> Result<(), std::io::Error> {
        let conf = self.gst_pipeline_conf();
        let mut f = File::options()
            .write(true)
            .create(true)
            .open(DEFAULT_GST_STREAM_CONF)?;

        f.write_all(conf.as_bytes())
    }
}

impl Default for JanusStream {
    fn default() -> Self {
        let media = JanusMedia {
            age_ms: 13385101,
            codec: "h264".into(),
            label: "label".into(),
            mid: "v1".into(),
            mindex: 0,
            port: 20001,
            rtpmap: "H264/90000".into(),
            pt: 96,
            _type: "video".into(),
        };
        let metadata = JanusStreamMetadata {
            path: "/dev/video0".into(),
            video_stream_src: VideoStreamSource::Device,
        };
        Self {
            description: "".into(),
            enabled: false,
            id: 0,
            media: vec![media],
            metadata: metadata,
            name: "".into(),
            _type: "".into(),
            viewers: 0,
        }
    }
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
pub struct MediaCommandRequest {
    service: String,
    janus_stream: JanusStream,
    command: MediaCommand,
}

impl MediaCommandRequest {
    fn build_response(&self, output: &Output) -> Result<MediaCommandResponse> {
        let data: HashMap<String, serde_json::Value> = HashMap::new();
        let res = match output.status.success() {
            true => {
                let detail = String::from_utf8(output.stdout.clone())?;
                MediaCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Ok,
                    detail: detail,
                    data,
                }
            }
            false => {
                let detail = String::from_utf8(output.stderr.clone())?;
                MediaCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail: detail,
                    data,
                }
            }
        };
        Ok(res)
    }

    fn start(&self) -> Result<MediaCommandResponse> {
        // write stream config before restarting service
        self.janus_stream.write_gst_pipeline_conf()?;

        let output = process::Command::new("sudo")
            .args(&["systemctl", "restart", &self.service])
            .output()?;
        self.build_response(&output)
    }

    fn stop(&self) -> Result<MediaCommandResponse> {
        let output = process::Command::new("sudo")
            .args(&["systemctl", "stop", &self.service])
            .output()?;
        self.build_response(&output)
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
pub struct MediaCommandResponse {
    request: Option<MediaCommandRequest>,
    status: ResponseStatus,
    detail: String,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandRequest(SystemctlCommandRequest),
    #[serde(rename = "pi.command.media")]
    MediaCommandRequest(MediaCommandRequest),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsResponse {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandResponse(SystemctlCommandResponse),
    #[serde(rename = "pi.command.media")]
    MediaCommandResponse(MediaCommandResponse),
}

impl NatsResponse {}

impl MessageHandler<NatsRequest, NatsResponse> for NatsRequest {
    fn handle(&self, request: &NatsRequest) -> Result<NatsResponse> {
        match request {
            NatsRequest::SystemctlCommandRequest(request) => match request.command {
                SystemctlCommand::ListEnabled => Ok(NatsResponse::SystemctlCommandResponse(
                    request.list_enabled()?,
                )),
                SystemctlCommand::Start => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.start()?))
                }
                SystemctlCommand::Stop => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.stop()?))
                }
                SystemctlCommand::Restart => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.restart()?))
                }
                SystemctlCommand::Status => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.status()?))
                }
                SystemctlCommand::Enable => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.enable()?))
                }
                SystemctlCommand::Disable => {
                    Ok(NatsResponse::SystemctlCommandResponse(request.disable()?))
                }
            },
            NatsRequest::MediaCommandRequest(request) => match request.command {
                MediaCommand::Start => Ok(NatsResponse::MediaCommandResponse(request.start()?)),
                MediaCommand::Stop => Ok(NatsResponse::MediaCommandResponse(request.stop()?)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conf_file() {
        let janus_stream = JanusStream::default();
        let conf = janus_stream.gst_pipeline_conf();
        let expected = r#"UDP_PORT=20001
        INPUT_PATH=/dev/video0
        VIDEO_STREAM_SRC=device"#;
        assert_eq!(expected, conf);
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

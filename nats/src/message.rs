use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::process;
use std::{fmt, io::Write};

use anyhow::Result;
use async_process::Output;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::CommandError;
use crate::util;

const DEFAULT_GST_STREAM_CONF: &str = "/var/run/printnanny/printnanny-gst-vision.conf";

pub trait MessageHandler<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug,
    Response: Serialize + DeserializeOwned + Debug,
{
    fn handle(&self, request: &Request) -> Result<Response>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JanusStreamMetadata {
    path: String,
    video_stream_src: VideoStreamSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Start,
    Stop,
    Restart,
    Status,
    Enable,
    Disable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandRequest {
    service: String,
    command: SystemctlCommand,
}

impl SystemctlCommandRequest {
    fn build_response(&self, output: &Output) -> Result<SystemctlCommandResponse> {
        let res = match output.status.success() {
            true => {
                let detail = String::from_utf8(output.stdout.clone())?;
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Ok,
                    detail: detail,
                    data: None,
                }
            }
            false => {
                let detail = String::from_utf8(output.stderr.clone())?;
                SystemctlCommandResponse {
                    request: Some(self.clone()),
                    status: ResponseStatus::Error,
                    detail: detail,
                    data: None,
                }
            }
        };
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
        let output = process::Command::new("sudo")
            .args(&["systemctl", "show", &self.service])
            .output()?;

        let mut res = self.build_response(&output)?;
        res.data = Some(util::systemctl_show_payload(res.detail.as_bytes())?);
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemctlCommandResponse {
    request: Option<SystemctlCommandRequest>,
    status: ResponseStatus,
    detail: String,
    data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandRequest(SystemctlCommandRequest),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsResponse {
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandResponse(SystemctlCommandResponse),
}

impl NatsResponse {}

impl MessageHandler<NatsRequest, NatsResponse> for NatsRequest {
    fn handle(&self, request: &NatsRequest) -> Result<NatsResponse> {
        match request {
            NatsRequest::SystemctlCommandRequest(request) => match request.command {
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
                _ => unimplemented!(),
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
}

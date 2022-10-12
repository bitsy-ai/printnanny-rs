use serde::{Deserialize, Serialize};
use std::fs::File;
use std::process::Command;
use std::{fmt, io::Write};

use crate::error::CommandError;

const DEFAULT_GST_STREAM_CONF: &str = "/var/run/printnanny/printnanny-gst-vision.conf";

pub trait MessageHandler {
    fn handle(&self) -> Result<(), CommandError>;
}

pub trait MessageResponse<Request, Response> {
    fn new(request: Option<Request>, status: ResponseStatus, detail: String) -> Response;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemctlCommand {
    Start,
    Stop,
    Restart,
    Status,
    Enable,
    Disable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcCommandRequest {
    subject: String,
    janus_stream: JanusStream,
    command: SystemctlCommand,
}

impl QcCommandRequest {
    fn start(&self) -> Result<(), CommandError> {
        // write conf file before restarting systemd unit
        self.janus_stream.write_gst_pipeline_conf()?;
        Command::new("sudo")
            .args(&["systemctl", "restart", "printnanny-gst-vision.service"])
            .output()?;
        Ok(())
    }
    fn stop(&self) -> Result<(), CommandError> {
        Command::new("sudo")
            .args(&["systemctl", "stop", "printnanny-gst-vision.service"])
            .output()?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QcCommandResponse {
    request: Option<QcCommandRequest>,
    status: ResponseStatus,
    detail: String,
}

impl MessageResponse<QcCommandRequest, QcCommandResponse> for QcCommandResponse {
    fn new(
        request: Option<QcCommandRequest>,
        status: ResponseStatus,
        detail: String,
    ) -> QcCommandResponse {
        QcCommandResponse {
            request,
            status,
            detail,
        }
    }
}

impl MessageHandler for QcCommandRequest {
    fn handle(&self) -> Result<(), crate::error::CommandError> {
        match self.command {
            SystemctlCommand::Start => self.start(),
            SystemctlCommand::Stop => self.stop(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcCommandRequest {
    subject: String,
    janus_stream: JanusStream,
    command: SystemctlCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemctlCommandRequest {
    service: String,
    command: SystemctlCommand,
}

impl SystemctlCommandRequest {
    fn start(&self) -> Result<(), CommandError> {
        // write conf file before restarting systemd unit
        self.janus_stream.write_gst_pipeline_conf()?;
        Command::new("sudo")
            .args(&["systemctl", "start", &self.service])
            .output()?;
        Ok(())
    }
    fn stop(&self) -> Result<(), CommandError> {
        Command::new("sudo")
            .args(&["systemctl", "stop", &self.service])
            .output()?;
        Ok(())
    }
    fn restart(&self) -> Result<(), CommandError> {
        Command::new("sudo")
            .args(&["systemctl", "restart", &self.service])
            .output()?;
        Ok(())
    }
}

impl MessageHandler for SystemctlCommandRequest {
    fn handle(&self) -> Result<(), crate::error::CommandError> {
        match self.command {
            SystemctlCommand::Start => self.start(),
            SystemctlCommand::Stop => self.stop(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemctlCommandResponse {
    request: Option<SystemctlCommandRequest>,
    status: ResponseStatus,
    detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject")]
pub enum NatsRequest {
    #[serde(rename = "pi.command.qc")]
    QcCommandRequest(QcCommandRequest),
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandRequest(SystemctlCommandRequest),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subject_pattern")]
pub enum NatsResponse {
    #[serde(rename = "pi.command.qc")]
    QcCommandResponse(QcCommandResponse),
    #[serde(rename = "pi.command.systemctl")]
    SystemctlCommandResponse(SystemctlCommandResponse),
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

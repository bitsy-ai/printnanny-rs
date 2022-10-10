use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{CommandError, NatsError};

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
pub enum NatsQcCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsQcCommandRequest {
    subject: String,
    janus_stream: JanusStream,
    command: NatsQcCommand,
}

impl NatsQcCommandRequest {
    fn start(&self) -> Result<(), CommandError> {
        Ok(())
    }
    fn stop(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NatsQcCommandResponse {
    request: Option<NatsQcCommandRequest>,
    status: ResponseStatus,
    detail: String,
}

impl MessageResponse<NatsQcCommandRequest, NatsQcCommandResponse> for NatsQcCommandResponse {
    fn new(
        request: Option<NatsQcCommandRequest>,
        status: ResponseStatus,
        detail: String,
    ) -> NatsQcCommandResponse {
        NatsQcCommandResponse {
            request,
            status,
            detail,
        }
    }
}

impl MessageHandler for NatsQcCommandRequest {
    fn handle(&self) -> Result<(), crate::error::CommandError> {
        match self.command {
            NatsQcCommand::Start => self.start(),
            NatsQcCommand::Stop => self.stop(),
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

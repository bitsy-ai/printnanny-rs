use crate::error::{CommandError, NatsError};
use log::info;
use serde::{Deserialize, Serialize};

pub trait MessageHandler {
    fn handle(&self) -> Result<(), CommandError> {
        Ok(())
    }
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
pub enum JanusStreamSource {
    File,
    Device,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JanusStreamMetadata {
    path: String,
    source: JanusStreamSource,
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
    pub fn handle(&self) -> Result<(), NatsError> {
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
        info!("Handling NatsQcCommandRequest {:?}", self);
        Ok(())
    }
}

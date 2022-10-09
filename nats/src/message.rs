use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NatsQcCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NatsQcCommandResult {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsQcCommandRequest {
    subject: String,
    stream_description: String,
    stream_id: String,
    command: NatsQcCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsQcCommandResponse {
    request: NatsQcCommandRequest,
    result: NatsQcCommandResult,
}

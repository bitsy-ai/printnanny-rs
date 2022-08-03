use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    pub uri: String,
    pub require_tls: bool,
    pub creds_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsJsonEvent {
    pub subject: String,
    pub payload: serde_json::value::Value,
}

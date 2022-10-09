use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsQcRequest {
    subject: String,
    #[serde(rename = "streamDescription")]
    stream_description: String,
    #[serde(rename = "streamId")]
    stream_id: String,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsJsonEvent {
    pub subject: String,
    pub payload: serde_json::value::Value,
}

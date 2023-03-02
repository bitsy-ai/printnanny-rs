use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GstMultiFileSinkMessage {
    pub filename: String,
    pub index: u32,
    pub timestamp: u64,
    #[serde(rename = "stream-time")]
    pub streamtime: u64,
    #[serde(rename = "running-time")]
    pub runningtime: u64,
    pub duration: u64,
    pub offset: u64,
    pub offset_end: u64,
}

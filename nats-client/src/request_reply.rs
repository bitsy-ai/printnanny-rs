use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;

// trait for handling NATS request / reply messages
#[async_trait]
pub trait NatsRequestHandler {
    type Request: Serialize + DeserializeOwned + Clone + Debug + NatsRequestHandler;
    type Reply: Serialize + DeserializeOwned + Clone + Debug;

    fn replace_subject_pattern(subject: &str, pattern: &str, replace: &str) -> String {
        // replace only first instance of pattern
        subject.replacen(pattern, replace, 1)
    }
    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Request>;
    async fn handle(&self) -> Result<Self::Reply>;
}

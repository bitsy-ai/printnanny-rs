use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;

// trait for handling one-way NATS event messages
#[async_trait]
pub trait NatsEventHandler {
    type Event: Serialize + DeserializeOwned + Clone + Debug + NatsEventHandler;

    fn replace_subject_pattern(subject: &str, pattern: &str, replace: &str) -> String {
        // replace only first instance of pattern
        subject.replacen(pattern, replace, 1)
    }
    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Event>;
    async fn handle(&self) -> Result<()>;
}

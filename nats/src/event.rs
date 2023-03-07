use std::fmt::Debug;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use log::warn;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_dbus::printnanny_asyncapi_models::VideoRecordingPart;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "subject_pattern")]
pub enum NatsEvent {
    // pi.{pi_id}.event.camera.recording.part
    #[serde(rename = "pi.{pi_id}.event.camera.recording.part")]
    VideoRecordingPart(VideoRecordingPart),
}

impl NatsEvent {
    async fn handle_video_recording_part(event: &VideoRecordingPart) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl NatsEventHandler for NatsEvent {
    type Event = NatsEvent;

    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Event> {
        match subject_pattern {
            "pi.{pi_id}.event.camera.recording.part" => {
                Ok(NatsEvent::VideoRecordingPart(serde_json::from_slice::<
                    VideoRecordingPart,
                >(
                    payload.as_ref()
                )?))
            }
            _ => Err(anyhow!(
                " NatsEventHandler not implemented for subject pattern {}",
                subject_pattern
            )),
        }
    }

    async fn handle(&self) -> Result<()> {
        match self {
            NatsEvent::VideoRecordingPart(event) => Self::handle_video_recording_part(event).await,
        }
    }
}

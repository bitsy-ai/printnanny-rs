use std::fmt::Debug;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_dbus::printnanny_asyncapi_models::VideoRecordingPart;
use printnanny_services::video_recording_sync::upload_video_recording_part;
use printnanny_settings::printnanny::PrintNannySettings;

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
        let settings = PrintNannySettings::new().await?;
        let sqlite_connection = settings.paths.db().display().to_string();
        upload_video_recording_part(event.into(), settings.cloud, sqlite_connection).await?;
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

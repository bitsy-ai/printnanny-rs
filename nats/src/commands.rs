use anyhow::Result;
use async_process::Command;
use bytes::Bytes;
use clap::ArgMatches;
use log::{debug, warn};
use printnanny_api_client::models::{self, PolymorphicPiEventRequest};
use printnanny_services::config::PrintNannyConfig;
use std::collections::HashMap;

use crate::util::to_nats_publish_subject;

pub fn build_status_payload(request: &PolymorphicPiEventRequest) -> Result<Bytes> {
    Ok(serde_json::ser::to_vec(request)?.into())
}

pub fn build_boot_status_payload(
    cmd: &models::polymorphic_pi_event_request::PiBootEventRequest,
    event_type: models::PiBootEventType,
    payload: Option<HashMap<String, serde_json::Value>>,
) -> Result<(String, Bytes)> {
    // command will be received on pi.$id.<topic>.commands
    // emit status event to pi.$id.<topic>.commands.$command_id
    let subject = to_nats_publish_subject(&cmd.pi, "boot", &event_type.to_string());
    let request = PolymorphicPiEventRequest::PiBootEventRequest(
        models::polymorphic_pi_event_request::PiBootEventRequest {
            subject: subject.clone(),
            payload,
            event_type,
            pi: cmd.pi,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject, b))
}

pub async fn handle_pi_boot_command(
    cmd: models::polymorphic_pi_event_request::PiBootEventRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match cmd.event_type {
        models::PiBootEventType::RebootCommand => {
            // publish RebootStarted event

            let (subject, req) =
                build_boot_status_payload(&cmd, models::PiBootEventType::RebootStarted, None)?;
            nats_client.publish(subject.clone(), req.into()).await?;
            debug!(
                "nats.publish subject={} event_type={:?}",
                &subject,
                models::PiBootEventType::RebootStarted
            );
            let output = Command::new("reboot").output().await?;
            match output.status.success() {
                // nothing to do, next event will be published on boot start
                true => (),
                false => {
                    // publish RebootError
                    let mut payload: HashMap<String, serde_json::Value> = HashMap::new();
                    payload.insert(
                        "exit_code".to_string(),
                        serde_json::to_value(output.status.code())?,
                    );
                    payload.insert(
                        "stdout".to_string(),
                        serde_json::Value::String(String::from_utf8(output.stdout)?),
                    );
                    payload.insert(
                        "stderr".to_string(),
                        serde_json::Value::String(String::from_utf8(output.stderr)?),
                    );
                    let (subject, req) = build_boot_status_payload(
                        &cmd,
                        models::PiBootEventType::RebootError,
                        Some(payload),
                    )?;

                    nats_client.publish(subject.clone(), req.into()).await?;
                    debug!(
                        "nats.publish subject={} event_type={:?}",
                        &subject,
                        models::PiBootEventType::RebootError
                    );
                }
            }
        }
        models::PiBootEventType::ShutdownCommand => {
            Command::new("shutdown").output().await?;
        }
        _ => warn!("No handler configured for msg={:?}", &cmd),
    };
    Ok(())
}

// pub async fn handle_pi_gstreamer_command(cmd: models::polymorphic_pi_event::PiGstreamerCommand, nats_client: &async_nats::Client) -> Result<()>{
//     match cmd.event_type {
//         models::PiGstreamerCommandType::Start => {

//             let output = Command::new("systemctl").args(&["start", "printnanny-cam"]).output().await?;

//             match output.status.success() {
//                 true => {
//                 }
//             }
//         },
//         models::PiGstreamerCommandType::Stop => {
//             Command::new("systemctl").args(&["stop", "printnanny-cam"]).await?
//         }
//     }
// }

pub async fn handle_incoming(
    msg: PolymorphicPiEventRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match msg {
        PolymorphicPiEventRequest::PiBootEventRequest(command) => {
            handle_pi_boot_command(command, nats_client).await?;
        }
        // models::PolymorphicPiEvent::PiGstreamerCommand(command) => handle_pi_gstreamer_command(command, nats_client).await?
        _ => warn!("No handler configured for msg={:?}", msg),
    };

    Ok(())
}

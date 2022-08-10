use anyhow::Result;
use async_process::Command;
use bytes::Bytes;
use log::{debug, warn};
use printnanny_api_client::models::{self, PolymorphicPiEventRequest};
use std::{collections::HashMap, fmt::format};

use crate::subjects;
pub fn build_status_payload(request: &PolymorphicPiEventRequest) -> Result<Bytes> {
    Ok(serde_json::ser::to_vec(request)?.into())
}

pub fn build_boot_status_payload(
    cmd: &models::polymorphic_pi_event_request::PiBootCommandRequest,
    event_type: models::PiBootStatusType,
    payload: Option<HashMap<String, serde_json::Value>>,
) -> Result<(String, Bytes)> {
    // command will be received on pi.$id.<topic>.commands
    // emit status event to pi.$id.<topic>.commands.$command_id
    let subject = stringify!(subjects::SUBJECT_STATUS_BOOT, pi_id = cmd.pi);

    let request = PolymorphicPiEventRequest::PiBootStatusRequest(
        models::polymorphic_pi_event_request::PiBootStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject.to_string(), b))
}

pub async fn handle_pi_boot_command(
    cmd: models::polymorphic_pi_event_request::PiBootCommandRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match cmd.event_type {
        models::PiBootCommandType::Reboot => {
            // publish RebootStarted event

            let (subject, req) =
                build_boot_status_payload(&cmd, models::PiBootStatusType::RebootStarted, None)?;
            nats_client.publish(subject.clone(), req).await?;
            debug!(
                "nats.publish event_type={:?}",
                models::PiBootStatusType::RebootStarted
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
                        models::PiBootStatusType::RebootError,
                        Some(payload),
                    )?;

                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish subject={} event_type={:?}",
                        &subject,
                        models::PiBootStatusType::RebootError
                    );
                }
            }
        }
        models::PiBootCommandType::Shutdown => {
            Command::new("shutdown").output().await?;
        }
        _ => warn!("No handler configured for msg={:?}", &cmd),
    };
    Ok(())
}

pub async fn handle_incoming(
    msg: PolymorphicPiEventRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match msg {
        PolymorphicPiEventRequest::PiBootCommandRequest(command) => {
            handle_pi_boot_command(command, nats_client).await?;
        }
        _ => warn!("No handler configured for msg={:?}", msg),
    };

    Ok(())
}

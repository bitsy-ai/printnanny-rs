use anyhow::Result;
use async_process::Command;
use bytes::Bytes;
use log::{debug, warn};
use std::{collections::HashMap, fmt::format};

use printnanny_api_client::models::{self, PolymorphicPiEventRequest};
use printnanny_services::swupdate::Swupdate;

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
    let subject = format!("pi.{pi_id}.status.boot", pi_id = cmd.pi);

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
                        "nats.publish event_type={:?}",
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

pub fn build_cam_status_payload(
    cmd: &models::polymorphic_pi_event_request::PiCamCommandRequest,
    event_type: models::PiCamStatusType,
    payload: Option<HashMap<String, serde_json::Value>>,
) -> Result<(String, Bytes)> {
    // command will be received on pi.$id.<topic>.commands
    // emit status event to pi.$id.<topic>.commands.$command_id
    let subject = format!("pi.{pi_id}.status.cam", pi_id = cmd.pi);

    let request = PolymorphicPiEventRequest::PiCamStatusRequest(
        models::polymorphic_pi_event_request::PiCamStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject.to_string(), b))
}

pub async fn handle_pi_cam_command(
    cmd: models::polymorphic_pi_event_request::PiCamCommandRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match cmd.event_type {
        models::PiCamCommandType::CamStart => {
            // publish CamStarted event
            let (subject, req) =
                build_cam_status_payload(&cmd, models::PiCamStatusType::CamStarted, None)?;
            nats_client.publish(subject.clone(), req).await?;
            debug!(
                "nats.publish event_type={:?}",
                models::PiCamStatusType::CamStarted
            );
            let output = Command::new("systemctl")
                .args(&["restart", "printnanny-cam"])
                .output()
                .await?;
            match output.status.success() {
                // publish CamStartedSuccess event
                true => {
                    let (subject, req) = build_cam_status_payload(
                        &cmd,
                        models::PiCamStatusType::CamStartSuccess,
                        None,
                    )?;
                    nats_client.publish(subject.clone(), req).await?;
                }
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
                    let (subject, req) = build_cam_status_payload(
                        &cmd,
                        models::PiCamStatusType::CamError,
                        Some(payload),
                    )?;

                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish event_type={:?}",
                        models::PiCamStatusType::CamError,
                    );
                }
            }
        }
        models::PiCamCommandType::CamStop => {
            let output = Command::new("systemctl")
                .args(&["stop", "printnanny-cam"])
                .output()
                .await?;
            match output.status.success() {
                // publish CamStartedSuccess event
                true => {
                    let (subject, req) =
                        build_cam_status_payload(&cmd, models::PiCamStatusType::CamStopped, None)?;
                    nats_client.publish(subject.clone(), req).await?;
                }
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
                    let (subject, req) = build_cam_status_payload(
                        &cmd,
                        models::PiCamStatusType::CamError,
                        Some(payload),
                    )?;

                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish event_type={:?}",
                        models::PiCamStatusType::CamError,
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn build_swupdate_status_payload(
    cmd: &models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest,
    event_type: models::PiSoftwareUpdateStatusType,
    payload: Option<HashMap<String, serde_json::Value>>,
) -> Result<(String, Bytes)> {
    // command will be received on pi.$id.<topic>.commands
    // emit status event to pi.$id.<topic>.commands.$command_id
    let subject = format!("pi.{pi_id}.status.swupdate", pi_id = cmd.pi);

    let request = PolymorphicPiEventRequest::PiSoftwareUpdateStatusRequest(
        models::polymorphic_pi_event_request::PiSoftwareUpdateStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
            version: cmd.version.clone(),
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject.to_string(), b))
}

pub async fn handle_pi_swupdate_command(
    cmd: models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match &cmd.event_type {
        models::PiSoftwareUpdateCommandType::Swupdate => {
            // publish SwupdateStarted event
            let (subject, req) = build_swupdate_status_payload(
                &cmd,
                models::PiSoftwareUpdateStatusType::SwupdateStarted,
                None,
            )?;
            nats_client.publish(subject.clone(), req).await?;
            debug!(
                "nats.publish event_type={:?}",
                models::PiSoftwareUpdateStatusType::SwupdateStarted
            );

            let swupdate = Swupdate::from(*cmd.payload.clone());
            let output = swupdate.run().await?;
            match output.status.success() {
                true => {
                    // publish SwupdateStarted event
                    let (subject, req) = build_swupdate_status_payload(
                        &cmd,
                        models::PiSoftwareUpdateStatusType::SwupdateSuccess,
                        None,
                    )?;
                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish event_type={:?}",
                        models::PiSoftwareUpdateStatusType::SwupdateSuccess
                    );
                }
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
                    let (subject, req) = build_swupdate_status_payload(
                        &cmd,
                        models::PiSoftwareUpdateStatusType::SwupdateError,
                        Some(payload),
                    )?;

                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish event_type={:?}",
                        models::PiSoftwareUpdateStatusType::SwupdateError
                    );
                }
            }
        }
        models::PiSoftwareUpdateCommandType::SwupdateRollback => {
            warn!("SwupdateRollback is not yet available")
        }
    }
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
        PolymorphicPiEventRequest::PiCamCommandRequest(command) => {
            handle_pi_cam_command(command, nats_client).await?;
        }
        PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(command) => {
            handle_pi_swupdate_command(command, nats_client).await?;
        }
        _ => warn!("No handler configured for msg={:?}", msg),
    };

    Ok(())
}

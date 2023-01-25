use anyhow::Result;
use async_process::Command;
use bytes::Bytes;
use chrono::prelude::{DateTime, Utc};
use log::{debug, warn};
use printnanny_api_client::models::{self, PolymorphicPiEventRequest};
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::swupdate::Swupdate;
use printnanny_settings::printnanny::PrintNannySettings;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

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
    let id = Some(Uuid::new_v4().to_string());
    let created_dt: DateTime<Utc> = SystemTime::now().into();
    let created_dt = Some(created_dt.to_rfc3339());

    let request = PolymorphicPiEventRequest::PiBootStatusRequest(
        models::polymorphic_pi_event_request::PiBootStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
            id,
            created_dt,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject, b))
}

pub async fn handle_pi_boot_command(
    cmd: models::polymorphic_pi_event_request::PiBootCommandRequest,
    reply: Option<String>,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match cmd.event_type {
        models::PiBootCommandType::Reboot => {
            // publish RebootStarted event

            let (subject, req) =
                build_boot_status_payload(&cmd, models::PiBootStatusType::RebootStarted, None)?;

            // publish to status topic
            nats_client.publish(subject.clone(), req.clone()).await?;

            debug!(
                "nats.publish event_type={:?}",
                models::PiBootStatusType::RebootStarted
            );
            let output = Command::new("reboot").output().await?;
            match output.status.success() {
                // nothing to do, next event will be published on boot start
                true => {
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req)
                            .await?;
                    }
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
                    let (subject, req) = build_boot_status_payload(
                        &cmd,
                        models::PiBootStatusType::RebootError,
                        Some(payload),
                    )?;
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
            let (subject, req) =
                build_boot_status_payload(&cmd, models::PiBootStatusType::ShutdownStarted, None)?;

            // publish to reply topic if present
            if reply.is_some() {
                nats_client
                    .publish(reply.as_ref().unwrap().to_string(), req.clone())
                    .await?;
            }
            // also publish to status topic
            nats_client.publish(subject.clone(), req).await?;

            debug!(
                "nats.publish event_type={:?}",
                models::PiBootStatusType::RebootStarted
            );
        }

        models::PiBootCommandType::SyncSettings => {
            // publish SyncSettings event
            let (subject, req) = build_boot_status_payload(
                &cmd,
                models::PiBootStatusType::SyncSettingsStarted,
                None,
            )?;

            //  publish to status topic
            nats_client.publish(subject.clone(), req).await?;
            let settings = PrintNannySettings::new().await?;
            let api = ApiService::from(&settings);
            let result = api.sync().await;

            match result {
                Ok(_) => {
                    // Publish SyncSettingSuccess event
                    let (subject, req) = build_boot_status_payload(
                        &cmd,
                        models::PiBootStatusType::SyncSettingsSuccess,
                        None,
                    )?;
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }
                    // also publish to status topic
                    nats_client.publish(subject.clone(), req).await?;
                }
                Err(e) => {
                    // Publish SyncSettingsError event

                    let mut payload: HashMap<String, serde_json::Value> = HashMap::new();
                    payload.insert(
                        "error".to_string(),
                        serde_json::Value::from(format!("{:?}", e)),
                    );
                    let payload = Some(payload);
                    let (subject, req) = build_boot_status_payload(
                        &cmd,
                        models::PiBootStatusType::SyncSettingsError,
                        payload,
                    )?;

                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }
                    // also publish to status topic
                    nats_client.publish(subject.clone(), req).await?;
                }
            }
        }
        _ => todo!(),
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
    let id = Some(Uuid::new_v4().to_string());
    let created_dt: DateTime<Utc> = SystemTime::now().into();
    let created_dt = Some(created_dt.to_rfc3339());

    let request = PolymorphicPiEventRequest::PiCamStatusRequest(
        models::polymorphic_pi_event_request::PiCamStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
            id,
            created_dt,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject, b))
}

pub async fn handle_pi_cam_command(
    cmd: models::polymorphic_pi_event_request::PiCamCommandRequest,
    reply: Option<String>,
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
            let output = Command::new("sudo")
                .args(["systemctl", "restart", "printnanny-cam.service"])
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
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

                    nats_client.publish(subject.clone(), req).await?;
                    debug!(
                        "nats.publish event_type={:?}",
                        models::PiCamStatusType::CamError,
                    );
                }
            }
        }
        models::PiCamCommandType::CamStop => {
            let output = Command::new("sudo")
                .args(["systemctl", "stop", "printnanny-cam.service"])
                .output()
                .await?;
            match output.status.success() {
                // publish CamStartedSuccess event
                true => {
                    let (subject, req) =
                        build_cam_status_payload(&cmd, models::PiCamStatusType::CamStopped, None)?;
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
    let id = Some(Uuid::new_v4().to_string());
    let created_dt: DateTime<Utc> = SystemTime::now().into();
    let created_dt = Some(created_dt.to_rfc3339());

    let request = PolymorphicPiEventRequest::PiSoftwareUpdateStatusRequest(
        models::polymorphic_pi_event_request::PiSoftwareUpdateStatusRequest {
            payload,
            event_type,
            pi: cmd.pi,
            version: cmd.version.clone(),
            id,
            created_dt,
        },
    );
    let b = build_status_payload(&request)?;

    Ok((subject, b))
}

pub async fn handle_pi_swupdate_command(
    cmd: models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest,
    reply: Option<String>,
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
            // publish to reply topic if present
            if reply.is_some() {
                nats_client
                    .publish(reply.as_ref().unwrap().to_string(), req.clone())
                    .await?;
            }

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
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
                    // publish to reply topic if present
                    if reply.is_some() {
                        nats_client
                            .publish(reply.as_ref().unwrap().to_string(), req.clone())
                            .await?;
                    }

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
    reply: Option<String>,
    nats_client: &async_nats::Client,
) -> Result<()> {
    match msg {
        PolymorphicPiEventRequest::PiBootCommandRequest(command) => {
            handle_pi_boot_command(command, reply, nats_client).await?;
        }
        PolymorphicPiEventRequest::PiCamCommandRequest(command) => {
            handle_pi_cam_command(command, reply, nats_client).await?;
        }
        PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(command) => {
            handle_pi_swupdate_command(command, reply, nats_client).await?;
        }
        _ => warn!("No handler configured for msg={:?}", msg),
    };

    Ok(())
}

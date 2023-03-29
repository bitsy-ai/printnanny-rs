use std::fmt::Debug;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use printnanny_api_client::models;
use serde::{Deserialize, Serialize};

use printnanny_nats_client::event::NatsEventHandler;
use printnanny_octoprint_models::{self, JobProgress};
use printnanny_services::printnanny_api::ApiService;
use printnanny_settings::printnanny::PrintNannySettings;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "subject_pattern")]
pub enum NatsEvent {
    #[serde(rename = "pi.{pi_id}.octoprint.event.server.startup")]
    OctoPrintServerStartup(printnanny_octoprint_models::OctoPrintServerStatusChanged),

    #[serde(rename = "pi.{pi_id}.octoprint.event.server.shutdown")]
    OctoPrintServerShutdown(printnanny_octoprint_models::OctoPrintServerStatusChanged),

    #[serde(rename = "pi.{pi_id}.octoprint.event.printer.status")]
    PrinterStatusChanged(printnanny_octoprint_models::PrinterStatusChanged),

    #[serde(rename = "pi.{pi_id}.octoprint.event.printer.job_progress")]
    JobProgressChanged(printnanny_octoprint_models::JobProgressChanged),

    #[serde(rename = "pi.{pi_id}.octoprint.event.printer.job_status")]
    JobStatusChanged(printnanny_octoprint_models::JobStatusChanged),

    #[serde(rename = "pi.{pi_id}.octoprint.event.gcode")]
    OctoPrintGcode(printnanny_octoprint_models::OctoPrintGcode),
}

impl NatsEvent {
    fn handle_octoprint_server_startup(
        event: &printnanny_octoprint_models::OctoPrintServerStatusChanged,
    ) -> Result<()> {
        info!("handle_octoprint_server_startup event={:?}", event);
        Ok(())
    }

    fn handle_octoprint_server_shutdown(
        event: &printnanny_octoprint_models::OctoPrintServerStatusChanged,
    ) -> Result<()> {
        info!("handle_octoprint_server_shutdown event={:?}", event);
        Ok(())
    }

    fn handle_octoprint_printer_status(
        event: &printnanny_octoprint_models::PrinterStatusChanged,
    ) -> Result<()> {
        info!("handle_octoprint_printer_status event={:?}", event);

        Ok(())
    }

    async fn handle_octoprint_job_status_changed(
        event: &printnanny_octoprint_models::JobStatusChanged,
    ) -> Result<()> {
        info!("handle_octoprint_job_status_changed event={:?}", event);
        Ok(())
    }

    async fn handle_octoprint_job_progress(
        event: &printnanny_octoprint_models::JobProgressChanged,
    ) -> Result<()> {
        info!("handle_octoprint_job_progress event={:?}", event);
        let settings = PrintNannySettings::new().await?;
        let sqlite_connection = settings.paths.db().display().to_string();
        let email_alert_settings =
            printnanny_edge_db::cloud::EmailAlertSettings::get(&sqlite_connection)?;

        let completion = event
            .progress
            .as_ref()
            .expect("JobProgress.progress expected to be some value, but got None")
            .completion
            .expect("JobProgress.progress.completion expected to be some value, but got None");

        if email_alert_settings.print_progress_enabled
            && completion % email_alert_settings.progress_percent as f64 == 0_f64
        {
            let api = ApiService::new(settings.cloud, sqlite_connection);
            let latest_snapshot_file = settings.paths.latest_snapshot_file();

            let alert = api
                .print_job_alert_create(
                    models::EventTypeEnum::PrintProgress,
                    models::EventSourceEnum::Octoprint,
                    latest_snapshot_file,
                )
                .await?;
            info!("Success! Created PrintJobAlert id={}", alert.id);
        }

        Ok(())
    }

    fn handle_octoprint_gcode(event: &printnanny_octoprint_models::OctoPrintGcode) -> Result<()> {
        info!("handle_octoprint_gcode event={:?}", event);
        Ok(())
    }
}

#[async_trait]
impl NatsEventHandler for NatsEvent {
    type Event = NatsEvent;

    fn deserialize_payload(subject_pattern: &str, payload: &Bytes) -> Result<Self::Event> {
        match subject_pattern {
            "pi.{pi_id}.octoprint.event.server.startup" => {
                Ok(NatsEvent::OctoPrintServerStartup(serde_json::from_slice::<
                    printnanny_octoprint_models::OctoPrintServerStatusChanged,
                >(
                    payload.as_ref()
                )?))
            }

            "pi.{pi_id}.octoprint.event.server.shutdown" => {
                Ok(NatsEvent::OctoPrintServerStartup(serde_json::from_slice::<
                    printnanny_octoprint_models::OctoPrintServerStatusChanged,
                >(
                    payload.as_ref()
                )?))
            }

            "pi.{pi_id}.octoprint.event.printer.status" => {
                Ok(NatsEvent::PrinterStatusChanged(serde_json::from_slice::<
                    printnanny_octoprint_models::PrinterStatusChanged,
                >(
                    payload.as_ref()
                )?))
            }

            "pi.{pi_id}.octoprint.event.printer.job_status" => {
                Ok(NatsEvent::JobStatusChanged(serde_json::from_slice::<
                    printnanny_octoprint_models::JobStatusChanged,
                >(
                    payload.as_ref()
                )?))
            }

            "pi.{pi_id}.octoprint.event.printer.job_progress" => {
                Ok(NatsEvent::JobProgressChanged(serde_json::from_slice::<
                    printnanny_octoprint_models::JobProgressChanged,
                >(
                    payload.as_ref()
                )?))
            }

            "pi.{pi_id}.octoprint.event.gcode" => {
                Ok(NatsEvent::OctoPrintGcode(serde_json::from_slice::<
                    printnanny_octoprint_models::OctoPrintGcode,
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
            NatsEvent::OctoPrintServerStartup(event) => {
                Self::handle_octoprint_server_startup(event)
            }
            NatsEvent::OctoPrintServerShutdown(event) => {
                Self::handle_octoprint_server_shutdown(event)
            }

            NatsEvent::PrinterStatusChanged(event) => Self::handle_octoprint_printer_status(event),

            NatsEvent::JobProgressChanged(event) => {
                Self::handle_octoprint_job_progress(event).await
            }

            NatsEvent::JobStatusChanged(event) => {
                Self::handle_octoprint_job_status_changed(event).await
            }

            NatsEvent::OctoPrintGcode(event) => Self::handle_octoprint_gcode(event),
        }
    }
}

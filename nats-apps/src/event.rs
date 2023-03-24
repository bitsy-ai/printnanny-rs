use std::fmt::Debug;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use serde::{Deserialize, Serialize};

use printnanny_nats_client::event::NatsEventHandler;
use printnanny_octoprint_models;

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
    JobProgress(printnanny_octoprint_models::JobProgress),

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

    fn handle_octoprint_job_progress(
        event: &printnanny_octoprint_models::JobProgress,
    ) -> Result<()> {
        info!("handle_octoprint_printer_status event={:?}", event);
        Ok(())
    }

    fn handle_octoprint_gcode(event: &printnanny_octoprint_models::OctoPrintGcode) -> Result<()> {
        info!(" handle_octoprint_gcode event={:?}", event);
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

            "pi.{pi_id}.octoprint.event.printer.job_progress" => {
                Ok(NatsEvent::JobProgress(serde_json::from_slice::<
                    printnanny_octoprint_models::JobProgress,
                >(payload.as_ref())?))
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

            NatsEvent::JobProgress(event) => Self::handle_octoprint_job_progress(event),

            NatsEvent::OctoPrintGcode(event) => Self::handle_octoprint_gcode(event),
        }
    }
}

use futures::prelude::*;
use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{debug, error, LevelFilter};
use printnanny_services::{config::PrintNannyConfig, error::PrintNannyConfigError};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

use printnanny_api_client::models;

use crate::error;
use crate::nats::NatsJsonEvent;

#[derive(Debug, Clone)]
pub struct EventCommand {
    args: ArgMatches,
    config: PrintNannyConfig,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl EventCommand {
    pub fn clap_command() -> Command<'static> {
        let app_name = "create";
        let app =
            Command::new(app_name)
                .author(crate_authors!())
                .about("Create new PrintNanny event/command")
                .subcommand_required(true)
                .arg_required_else_help(true)
                // emit a boot event
                .subcommand(Command::new("boot").arg_required_else_help(true).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiBootEventType)),
                ));

        app
    }

    pub fn new(args: &ArgMatches) -> Result<Self, PrintNannyConfigError> {
        let config = PrintNannyConfig::new().unwrap();
        config.try_check_license()?;
        return Ok(Self {
            args: args.clone(),
            config,
        });
    }

    fn boot_subject(&self, pi_id: &i32) -> String {
        return format!("pi.{}.boot", pi_id);
    }

    pub async fn handle_boot(&self, sub_args: &ArgMatches) -> Result<()> {
        // serialize payload
        let event_type: models::PiBootEventType = *sub_args
            .get_one::<models::PiBootEventType>("event_type")
            .unwrap();

        let pi_id = self.config.pi.as_ref().unwrap().id;
        let subject = self.boot_subject(&pi_id);
        let req = models::PiBootEventRequest {
            subject: subject.clone(),
            pi: pi_id,
            event_type,
            payload: None,
        };
        // establish connection to unix socket
        self.publish(
            &subject,
            &event_type.to_string(),
            serde_json::to_value(req)?,
        )
        .await?;
        Ok(())
    }

    pub async fn publish(
        &self,
        subject: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let socket = &self.config.paths.events_socket;
        // open a connection to unix socket
        let stream = UnixStream::connect(socket).await?;
        // Delimit frames using a length header
        let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

        let event = NatsJsonEvent {
            subject: subject.to_string(),
            payload,
        };

        // Serialize frames with JSON
        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<NatsJsonEvent>::default(),
        );
        serialized.send(event).await?;
        debug!(
            "Emitted event subject={} socket={} value={}",
            &subject,
            socket.display(),
            &event_type
        );
        Ok(())
    }

    pub fn handle_swupdate(&self, sub_args: &ArgMatches) -> Result<()> {
        // serialize payload
        // establish connection to unix socket

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        // check unix socket exists
        let socket = &self.config.paths.events_socket;

        match socket.exists() {
            true => {
                match self.args.subcommand().unwrap() {
                    ("boot", sub_args) => self.handle_boot(sub_args).await?,
                    _ => error!("Invalid command"),
                };
                Ok(())
            }
            false => Err(error::PublishError::UnixSocketNotFound {
                path: socket.display().to_string(),
            }),
        }?;
        Ok(())
    }
}

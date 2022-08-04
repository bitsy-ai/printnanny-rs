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
    socket: PathBuf,
    args: ArgMatches,
    config: PrintNannyConfig,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl EventCommand {
    pub fn clap_command() -> Command<'static> {
        let app_name = "printnanny-event";
        let app =
            Command::new(app_name)
                .author(crate_authors!())
                // .propagate_version(true)
                .about("Emit PrintNanny event via Unix socket")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .arg(
                    Arg::new("v")
                        .short('v')
                        .multiple_occurrences(true)
                        .help("Sets the level of verbosity"),
                )
                .arg(
                    Arg::new("socket")
                        .long("socket")
                        .default_value(".tmp/events.sock")
                        .takes_value(true)
                        .help("Publish data to Unix socket"),
                )
                // emit a boot event
                .subcommand(Command::new("boot").arg_required_else_help(true).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiBootEventType)),
                ));

        app
    }

    pub fn new(args: ArgMatches) -> Result<Self, PrintNannyConfigError> {
        let socket = args
            .value_of("socket")
            .expect("--socket is required")
            .into();
        let verbosity = args.occurrences_of("v");
        let mut builder = Builder::new();
        match verbosity {
            0 => {
                builder.filter_level(LevelFilter::Warn).init();
            }
            1 => {
                builder.filter_level(LevelFilter::Info).init();
            }
            2 => {
                builder.filter_level(LevelFilter::Debug).init();
            }
            _ => builder.filter_level(LevelFilter::Trace).init(),
        };
        let config = PrintNannyConfig::new().unwrap();
        config.try_check_license()?;
        return Ok(Self {
            socket,
            args,
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
        // open a connection to unix socket
        let stream = UnixStream::connect(&self.config.paths.events_socket).await?;
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
            self.socket.display(),
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
        match &self.socket.exists() {
            true => {
                match self.args.subcommand().unwrap() {
                    ("boot", sub_args) => self.handle_boot(sub_args).await?,
                    _ => error!("Invalid command"),
                };
                Ok(())
            }
            false => Err(error::PublishError::UnixSocketNotFound {
                path: self.socket.display().to_string(),
            }),
        }?;
        Ok(())
    }
}

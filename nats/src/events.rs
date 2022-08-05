use futures::prelude::*;

use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command};
use log::{debug, error};
use printnanny_services::{config::PrintNannyConfig, error::PrintNannyConfigError};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

use printnanny_api_client::models;
use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;

use crate::error;
use crate::util::to_nats_publish_subject;

#[derive(Debug, Clone)]
pub struct EventPublisher {
    args: ArgMatches,
    config: PrintNannyConfig,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl EventPublisher {
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

    pub async fn handle_boot(&self, sub_args: &ArgMatches) -> Result<()> {
        // serialize payload
        let event_type: models::PiBootEventType = *sub_args
            .get_one::<models::PiBootEventType>("event_type")
            .unwrap();

        let pi_id = self.config.pi.as_ref().unwrap().id;
        let subject = to_nats_publish_subject(&pi_id, "boot", &event_type.to_string());

        let req = PolymorphicPiEventRequest::PiBootEventRequest(
            models::polymorphic_pi_event_request::PiBootEventRequest {
                subject: subject.clone(),
                pi: pi_id,
                event_type,
                payload: None,
            },
        );
        // establish connection to unix socket
        self.publish(subject, req).await?;
        Ok(())
    }

    pub async fn publish(&self, subject: String, payload: PolymorphicPiEventRequest) -> Result<()> {
        let socket = &self.config.paths.events_socket;
        // open a connection to unix socket
        let stream = UnixStream::connect(socket).await?;
        // Delimit frames using a length header
        let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

        // Serialize frames with JSON
        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<(String, PolymorphicPiEventRequest)>::default(),
        );
        serialized.send((subject.clone(), payload)).await?;
        debug!(
            "Emitted event to subject={} to socket={}",
            &subject,
            socket.display(),
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

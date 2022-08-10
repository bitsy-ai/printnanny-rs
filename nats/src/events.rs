use futures::prelude::*;

use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command};
use log::{debug, error};
use printnanny_api_client::models;
use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;
use printnanny_services::{config::PrintNannyConfig, error::PrintNannyConfigError};
use serde::{Deserialize, Serialize};
// use serde_variant::to_variant_name;
use tokio::net::UnixStream;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

use crate::error;
use crate::util::to_nats_publish_subject;

pub enum CommandNames {}

#[derive(Debug, Clone)]
pub struct EventPublisher {
    args: ArgMatches,
    config: PrintNannyConfig,
}
pub trait EventPublisherCli {
    fn clap_command() -> clap::Command<'static>;
    fn new(args: &ArgMatches) -> Result<Self, PrintNannyConfigError>
    where
        Self: Sized;
    fn publish(&self, subject: String, payload: PolymorphicPiEventRequest) -> Result<()>;
    fn run(&self) -> Result<()>;
}

impl EventPublisher {
    // initialize EventPublisher from clap::Command ArgMatches
    pub fn new(args: &ArgMatches) -> Result<Self, PrintNannyConfigError> {
        let config = PrintNannyConfig::new().unwrap();
        config.try_check_license()?;
        return Ok(Self {
            args: args.clone(),
            config,
        });
    }
    pub fn clap_command() -> Command<'static> {
        let app_name = "nats-publisher";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Issue command via NATs")
            .arg_required_else_help(true)
            .arg(Arg::new("topic").required(true).value_parser(
                clap::builder::PossibleValuesParser::new([
                    "pi.{pi_id}.command.boot",
                    "pi.{pi_id}.command.cam",
                    "pi.{pi_id}.command.swupdate",
                    "pi.{pi_id}.status.boot",
                    "pi.{pi_id}.status.cam",
                    "pi.{pi_id}.status.swupdate",
                ]),
            ))
            .arg(
                Arg::new("pi.{pi_id}.command.boot")
                    .required_if_eq("topic", "pi.{pi_id}.command.boot")
                    .value_parser(value_parser!(models::PiBootCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new("pi.{pi_id}.status.boot")
                    .required_if_eq("topic", "pi.{pi_id}.status.boot")
                    .value_parser(value_parser!(models::PiBootStatusType))
                    .group("event_type"),
            )
            .arg(
                Arg::new("pi.{pi_id}.command.cam")
                    .required_if_eq("topic", "pi.{pi_id}.command.cam")
                    .value_parser(value_parser!(models::PiCamCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new("pi.{pi_id}.status.cam")
                    .required_if_eq("topic", "pi.{pi_id}.status.cam")
                    .value_parser(value_parser!(models::PiCamStatusType))
                    .group("event_type"),
            )
            .arg(
                Arg::new("pi.{pi_id}.command.swupdate")
                    .required_if_eq("topic", "pi.{pi_id}.command.swupdate")
                    .value_parser(value_parser!(models::PiSoftwareUpdateCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new("pi.{pi_id}.status.swupdate")
                    .required_if_eq("topic", "pi.{pi_id}.status.swupdate")
                    .value_parser(value_parser!(models::PiSoftwareUpdateStatusType))
                    .group("event_type"),
            );
        app
    }

    // write content-length delimited frames to Unix socket (PrintNanny events.sock)
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

    // check unix socket is available for writing
    fn socket_ok(&self) -> Result<(), error::PublishError> {
        let socket = &self.config.paths.events_socket;
        match socket.exists() {
            true => Ok(()),
            false => Err(error::PublishError::UnixSocketNotFound {
                path: socket.display().to_string(),
            }),
        }
    }

    pub async fn run(self) -> Result<()> {
        self.socket_ok()?;

        let pi_id = self
            .config
            .pi
            .expect("Failed to read PrintNannyConfig.pi.id")
            .id;

        let socket = &self.config.paths.events_socket;
        let topic = self
            .args
            .get_one::<String>("topic")
            .expect("topic is required");

        let (subject, payload) = match topic.as_str() {
            "pi.{pi_id}.command.boot" => {
                let event_type = self
                    .args
                    .get_one::<models::PiBootCommandType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiBootCommandRequest(
                        models::polymorphic_pi_event_request::PiBootCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            "pi.{pi_id}.command.cam" => {
                let event_type = self
                    .args
                    .get_one::<models::PiCamCommandType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiCamCommandRequest(
                        models::polymorphic_pi_event_request::PiCamCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            "pi.{pi_id}.command.swupdate" => {
                let event_type = self
                    .args
                    .get_one::<models::PiSoftwareUpdateCommandType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            "pi.{pi_id}.status.boot" => {
                let event_type = self
                    .args
                    .get_one::<models::PiBootStatusType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiBootStatusType(
                        models::polymorphic_pi_event_request::PiBootStatusType {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            // "pi.{pi_id}.status.cam" => {}
            // "pi.{pi_id}.status.swupdate" => {}
            _ => panic!("Invalid topic: {}", &topic),
        };
        println!("Serialized subject={} payload={:?}", subject, payload);

        Ok(())
    }
}

// implement EventPublisher for PiCommand events

// implement EventPublisher for PiStatus events

// impl EventPublisher {

// }

// impl EventPublisherV1 {
//     pub fn status_clap_command() -> Command<'static> {
//         let app_name = "send-status";
//         let app =
//             Command::new(app_name)
//                 .author(crate_authors!())
//                 .about("Publish status event via NATs")
//                 .subcommand_required(true)
//                 .arg_required_else_help(true)
//                 // emit a boot status event
//                 .subcommand(Command::new("boot").arg_required_else_help(true).arg(
//                     Arg::new("event_type").value_parser(value_parser!(models::PiBootStatusType)),
//                 ));

//         app
//     }

//     pub fn remote_clap_command() -> Command<'static> {
//         let app_name: &str = "send-command";
//         let app = Command::new(app_name)
//             .author(crate_authors!())
//             .about("Publish remote control command via NATs")
//             .subcommand_required(true)
//             .arg_required_else_help(true)
//             // emit a boot status event
//             .subcommand(Command::new("boot").arg_required_else_help(true).arg(
//                 Arg::new("event_type").value_parser(value_parser!(models::PiBootCommandType)),
//             ));

//         app
//     }

//     pub fn new(args: &ArgMatches) -> Result<Self, PrintNannyConfigError> {
//         let config = PrintNannyConfig::new().unwrap();
//         config.try_check_license()?;
//         return Ok(Self {
//             args: args.clone(),
//             config,
//         });
//     }

//     pub async fn handle_boot_status(&self, sub_args: &ArgMatches) -> Result<()> {
//         // serialize payload
//         let event_type: models::PiBootStatusType = *sub_args
//             .get_one::<models::PiBootStatusType>("event_type")
//             .unwrap();

//         let pi_id = self.config.pi.as_ref().unwrap().id;
//         let subject = to_nats_publish_subject(&pi_id, "boot", &event_type.to_string());

//         let req = PolymorphicPiEventRequest::PiBootStatusRequest(
//             models::polymorphic_pi_event_request::PiBootStatusRequest {
//                 subject: subject.clone(),
//                 pi: pi_id,
//                 event_type,
//                 payload: None,
//             },
//         );
//         // establish connection to unix socket
//         self.publish(subject, req).await?;
//         Ok(())
//     }

//     pub async fn publish(&self, subject: String, payload: PolymorphicPiEventRequest) -> Result<()> {
//         let socket = &self.config.paths.events_socket;
//         // open a connection to unix socket
//         let stream = UnixStream::connect(socket).await?;
//         // Delimit frames using a length header
//         let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

//         // Serialize frames with JSON
//         let mut serialized = tokio_serde::SymmetricallyFramed::new(
//             length_delimited,
//             tokio_serde::formats::SymmetricalJson::<(String, PolymorphicPiEventRequest)>::default(),
//         );
//         serialized.send((subject.clone(), payload)).await?;
//         debug!(
//             "Emitted event to subject={} to socket={}",
//             &subject,
//             socket.display(),
//         );
//         Ok(())
//     }

//     pub fn handle_swupdate(&self, sub_args: &ArgMatches) -> Result<()> {
//         // serialize payload
//         // establish connection to unix socket

//         Ok(())
//     }

// pub async fn run(&self) -> Result<()> {
//     // check unix socket exists
//     let socket = &self.config.paths.events_socket;

//     match socket.exists() {
//         true => {
//             match self.args.subcommand().unwrap() {
//                 ("boot", sub_args) => self.handle_boot(sub_args).await?,
//                 _ => error!("Invalid command"),
//             };
//             Ok(())
//         }
//         false => Err(error::PublishError::UnixSocketNotFound {
//             path: socket.display().to_string(),
//         }),
//     }?;
//     Ok(())
// }
// }

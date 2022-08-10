use futures::prelude::*;

use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command};
use log::debug;
use printnanny_api_client::models;
use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;
use printnanny_services::{config::PrintNannyConfig, error::PrintNannyConfigError};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

use crate::error;
use crate::subjects;

#[derive(Debug, Clone)]
pub struct EventPublisher {
    args: ArgMatches,
    config: PrintNannyConfig,
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
                    subjects::SUBJECT_COMMAND_BOOT,
                    subjects::SUBJECT_STATUS_BOOT,
                    subjects::SUBJECT_COMMAND_CAM,
                    subjects::SUBJECT_STATUS_CAM,
                    subjects::SUBJECT_COMMAND_SWUPDATE,
                    subjects::SUBJECT_STATUS_SWUPDATE,
                ]),
            ))
            .arg(
                Arg::new(subjects::SUBJECT_COMMAND_BOOT)
                    .required_if_eq("topic", subjects::SUBJECT_COMMAND_BOOT)
                    .value_parser(value_parser!(models::PiBootCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new(subjects::SUBJECT_STATUS_BOOT)
                    .required_if_eq("topic", subjects::SUBJECT_STATUS_BOOT)
                    .value_parser(value_parser!(models::PiBootStatusType))
                    .group("event_type"),
            )
            .arg(
                Arg::new(subjects::SUBJECT_COMMAND_CAM)
                    .required_if_eq("topic", subjects::SUBJECT_COMMAND_CAM)
                    .value_parser(value_parser!(models::PiCamCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new(subjects::SUBJECT_STATUS_CAM)
                    .required_if_eq("topic", subjects::SUBJECT_STATUS_CAM)
                    .value_parser(value_parser!(models::PiCamStatusType))
                    .group("event_type"),
            )
            .arg(
                Arg::new(subjects::SUBJECT_COMMAND_SWUPDATE)
                    .required_if_eq("topic", subjects::SUBJECT_COMMAND_SWUPDATE)
                    .value_parser(value_parser!(models::PiSoftwareUpdateCommandType))
                    .group("event_type"),
            )
            .arg(
                Arg::new(subjects::SUBJECT_STATUS_SWUPDATE)
                    .required_if_eq("topic", subjects::SUBJECT_STATUS_SWUPDATE)
                    .value_parser(value_parser!(models::PiSoftwareUpdateStatusType))
                    .group("event_type"),
            );
        app
    }

    // write content-length delimited frames to Unix socket (PrintNanny events.sock)
    pub async fn publish(&self, subject: &str, payload: &PolymorphicPiEventRequest) -> Result<()> {
        let socket = &self.config.paths.events_socket;
        // open a connection to unix socket
        let stream = UnixStream::connect(socket).await?;
        // Delimit frames using a length header
        let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

        // Serialize frames with JSON
        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<(String, &PolymorphicPiEventRequest)>::default(
            ),
        );
        serialized.send((subject.to_string(), payload)).await?;
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
            .as_ref()
            .expect("Failed to read PrintNannyConfig.pi.id")
            .id;

        let topic = self
            .args
            .get_one::<String>("topic")
            .expect("topic is required");

        let (subject, payload) = match topic.as_str() {
            subjects::SUBJECT_COMMAND_BOOT => {
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
            subjects::SUBJECT_COMMAND_CAM => {
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
            subjects::SUBJECT_COMMAND_SWUPDATE => {
                let version = self
                    .args
                    .get_one::<String>("version")
                    .expect("version is required");
                let event_type = self
                    .args
                    .get_one::<models::PiSoftwareUpdateCommandType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest {
                            version: version.to_string(),
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            subjects::SUBJECT_STATUS_BOOT => {
                let event_type = self
                    .args
                    .get_one::<models::PiBootStatusType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiBootStatusRequest(
                        models::polymorphic_pi_event_request::PiBootStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            subjects::SUBJECT_STATUS_CAM => {
                let event_type = self
                    .args
                    .get_one::<models::PiCamStatusType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiCamStatusRequest(
                        models::polymorphic_pi_event_request::PiCamStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            subjects::SUBJECT_STATUS_SWUPDATE => {
                let version = self
                    .args
                    .get_one::<String>("version")
                    .expect("version is required");
                let event_type = self
                    .args
                    .get_one::<models::PiSoftwareUpdateStatusType>("event_type")
                    .expect("Invalid event_type");
                (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiSoftwareUpdateStatusRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateStatusRequest {
                            version: version.to_string(),
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                )
            }
            _ => panic!("Invalid topic: {}", &topic),
        };

        self.publish(&subject, &payload).await
    }
}

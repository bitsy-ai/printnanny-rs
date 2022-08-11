use futures::prelude::*;
use std::collections::HashMap;
use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command, ValueEnum};
use log::debug;
use printnanny_api_client::models;
use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;
use printnanny_api_client::models::polymorphic_octo_print_event_request::PolymorphicOctoPrintEventRequest;
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum PayloadFormat {
    Json,
    // TODO: serialize raw
    // Bytes,
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
        let app =
            Command::new(app_name)
                .author(crate_authors!())
                .about("Issue command via NATs")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .arg(Arg::new("subject").long("subject"))
                // begin octoprint topics
                .subcommand(
                    Command::new(subjects::SUBJECT_OCTOPRINT_SERVER)
                        .arg(Arg::new("event_type").value_parser(value_parser!(models::OctoPrintServerStatusType)))
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                .subcommand(
                    Command::new(subjects::SUBJECT_OCTOPRINT_CLIENT)
                        .arg(Arg::new("event_type").value_parser(value_parser!(models::OctoPrintClientStatusType)))
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                .subcommand(
                    Command::new(subjects::SUBJECT_OCTOPRINT_PRINTER_STATUS)
                        .arg(Arg::new("event_type").value_parser(value_parser!(models::OctoPrintPrinterStatusType)))
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                .subcommand(
                    Command::new(subjects::SUBJECT_OCTOPRINT_PRINT_JOB)
                        .arg(Arg::new("event_type").value_parser(value_parser!(models::OctoPrintPrintJobStatusType)))
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                // end octoprint topics
                // begin repetier topics
                .subcommand(
                    Command::new(subjects::SUBJECT_REPETIER)
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                // end repetier topics
                // begin moonraker topics
                .subcommand(
                    Command::new(subjects::SUBJECT_MOONRAKER)
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .takes_value(true)
                                .value_parser(value_parser!(PayloadFormat))
                                .default_value("json")
                                .help("Payload format"),
                        )
                        .arg(Arg::new("payload").long("payload").help("UTF-8 encoded JSON payload")),
                )
                // end moonraker topics
                // begin PrintNanny Pi topics
                .subcommand(Command::new(subjects::SUBJECT_COMMAND_BOOT).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiBootCommandType)),
                ))
                .subcommand(Command::new(subjects::SUBJECT_STATUS_BOOT).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiBootStatusType)),
                ))
                .subcommand(Command::new(subjects::SUBJECT_COMMAND_CAM).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiCamCommandType)),
                ))
                .subcommand(Command::new(subjects::SUBJECT_STATUS_CAM).arg(
                    Arg::new("event_type").value_parser(value_parser!(models::PiCamStatusType)),
                ))
                .subcommand(
                    Command::new(subjects::SUBJECT_COMMAND_SWUPDATE)
                        .arg(
                            Arg::new("event_type")
                                .value_parser(value_parser!(models::PiSoftwareUpdateCommandType)),
                        )
                        .arg(
                            Arg::new("wic_tarball_url")
                                .long("--wic-tarball-url")
                                .required(true),
                        )
                        .arg(
                            Arg::new("wic_bmap_url")
                                .long("--wic-bmap-url")
                                .required(true),
                        )
                        .arg(
                            Arg::new("manifest_url")
                                .long("--manifest-url")
                                .required(true),
                        )
                        .arg(Arg::new("swu_url").long("--swu-url").required(true))
                        .arg(Arg::new("version_id").long("--version-id").required(true))
                        .arg(Arg::new("version").long("--version").required(true))
                        .arg(
                            Arg::new("version_codename")
                                .long("--version-codename")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new(subjects::SUBJECT_STATUS_SWUPDATE).arg(
                        Arg::new("event_type")
                            .value_parser(value_parser!(models::PiSoftwareUpdateStatusType)),
                    ),
                );
        app
    }

    // write content-length delimited frames to Unix socket (PrintNanny events.sock)
    pub async fn publish_pi_event(&self, subject: &str, payload: &PolymorphicPiEventRequest) -> Result<()> {
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

    // write content-length delimited frames to Unix socket (PrintNanny events.sock)
    pub async fn publish_octoprint_event(&self, subject: &str, payload: &PolymorphicOctoPrintEventRequest) -> Result<()> {
        let socket = &self.config.paths.events_socket;
        // open a connection to unix socket
        let stream = UnixStream::connect(socket).await?;
        // Delimit frames using a length header
        let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

        // Serialize frames with JSON
        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<(String, &PolymorphicOctoPrintEventRequest)>::default(
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

        match self.args.subcommand().unwrap() {
            (subjects::SUBJECT_COMMAND_BOOT, subargs) => {
                let event_type = subargs
                    .get_one::<models::PiBootCommandType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_COMMAND_BOOT, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiBootCommandRequest(
                        models::polymorphic_pi_event_request::PiBootCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await
            }
            (subjects::SUBJECT_COMMAND_CAM, subargs) => {
                let event_type = subargs
                    .get_one::<models::PiCamCommandType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_COMMAND_CAM, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiCamCommandRequest(
                        models::polymorphic_pi_event_request::PiCamCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await

            }
            (subjects::SUBJECT_COMMAND_SWUPDATE, subargs) => {
                let version = subargs
                    .get_one::<String>("version")
                    .expect("version is required")
                    .to_string();
                let event_type = self
                    .args
                    .get_one::<models::PiSoftwareUpdateCommandType>("event_type")
                    .expect("Invalid event_type");

                let wic_tarball_url = self
                    .args
                    .get_one::<String>("wic_tarball_url")
                    .expect("--wic-tarball-url is required")
                    .into();
                let wic_bmap_url = self
                    .args
                    .get_one::<String>("wic_bmap_url")
                    .expect("--wic-bmap-url is required")
                    .into();
                let manifest_url = self
                    .args
                    .get_one::<String>("manifest_url")
                    .expect("--manifest-url is required")
                    .into();
                let swu_url = self
                    .args
                    .get_one::<String>("swu_url")
                    .expect("--swu-url is required")
                    .into();
                let version_id = self
                    .args
                    .get_one::<String>("version_id")
                    .expect("--version-id is required")
                    .into();
                let version_codename = self
                    .args
                    .get_one::<String>("version_codename")
                    .expect("--version-codename is required")
                    .into();

                let payload =
                    models::pi_software_update_payload_request::PiSoftwareUpdatePayloadRequest {
                        version: version.clone(),
                        version_id,
                        version_codename,
                        wic_tarball_url,
                        wic_bmap_url,
                        manifest_url,
                        swu_url,
                    };

                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_COMMAND_SWUPDATE, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest {
                            version,
                            event_type: *event_type,
                            pi: pi_id,
                            payload: Box::new(payload),
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await
            }
            (subjects::SUBJECT_STATUS_BOOT, subargs) => {
                let event_type = subargs
                    .get_one::<models::PiBootStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_STATUS_BOOT, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiBootStatusRequest(
                        models::polymorphic_pi_event_request::PiBootStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await


            }
            (subjects::SUBJECT_STATUS_CAM, subargs) => {
                let event_type = subargs
                    .get_one::<models::PiCamStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_STATUS_CAM, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiCamStatusRequest(
                        models::polymorphic_pi_event_request::PiCamStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await

            }
            (subjects::SUBJECT_STATUS_SWUPDATE, subargs) => {
                let version = subargs
                    .get_one::<String>("version")
                    .expect("version is required");
                let event_type = self
                    .args
                    .get_one::<models::PiSoftwareUpdateStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_STATUS_SWUPDATE, pi_id = pi_id).to_string(),
                    PolymorphicPiEventRequest::PiSoftwareUpdateStatusRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateStatusRequest {
                            version: version.to_string(),
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await

            }
            // begin octoprint subject handlers
            // pi.{pi_id}.octoprint.client
            (subjects::SUBJECT_OCTOPRINT_CLIENT, subargs) => {
                let payload = subargs.get_one::<String>("payload").expect("--payload is required");
                let payload = serde_json::from_str::<models::OctoPrintClientStatusPayloadRequest>(payload)?;
                let octoprint_server = self
                    .config
                    .octoprint
                    .as_ref()
                    .expect("Failed to readPrintNannyConfig.octoprint")
                    .server
                    .as_ref()
                    .expect("Failed to read PrintNannyConfig.octoprint.server")
                    .id;
                let event_type = self
                    .args
                    .get_one::<models::OctoPrintClientStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_OCTOPRINT_CLIENT, pi_id = pi_id).to_string(), 
                    PolymorphicOctoPrintEventRequest::OctoPrintClientStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintClientStatusRequest{
                        payload: Box::new(payload),
                        pi: pi_id,
                        event_type: *event_type,
                        octoprint_server
                    })
                );
                self.publish_octoprint_event(&subject, &payload).await

            }
            // pi.{pi_id}.octoprint.client
            (subjects::SUBJECT_OCTOPRINT_PRINT_JOB, subargs) => {
                let payload = subargs.get_one::<String>("payload").expect("--payload is required");
                let payload = serde_json::from_str::<models::OctoPrintPrintJobPayloadRequest>(payload)?;
                let octoprint_server = self
                    .config
                    .octoprint
                    .as_ref()
                    .expect("Failed to readPrintNannyConfig.octoprint")
                    .server
                    .as_ref()
                    .expect("Failed to read PrintNannyConfig.octoprint.server")
                    .id;
                let event_type = self
                    .args
                    .get_one::<models::OctoPrintPrintJobStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_OCTOPRINT_PRINT_JOB, pi_id = pi_id).to_string(), 
                    PolymorphicOctoPrintEventRequest::OctoPrintPrintJobStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintPrintJobStatusRequest{
                        payload: Box::new(payload),
                        pi: pi_id,
                        event_type: *event_type,
                        octoprint_server
                    })
                );
                self.publish_octoprint_event(&subject, &payload).await

            }

            // pi.{pi_id}.octoprint.server
            (subjects::SUBJECT_OCTOPRINT_SERVER, subargs) => {
                let payload = subargs.get_one::<String>("payload");
                let payload = match payload {
                    Some(data) => Some(serde_json::from_str::<HashMap<String, serde_json::Value>>(data)?),
                    None => None
                };
                let octoprint_server = self
                    .config
                    .octoprint
                    .as_ref()
                    .expect("Failed to readPrintNannyConfig.octoprint")
                    .server
                    .as_ref()
                    .expect("Failed to read PrintNannyConfig.octoprint.server")
                    .id;
                let event_type = self
                    .args
                    .get_one::<models::OctoPrintServerStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_OCTOPRINT_SERVER, pi_id = pi_id).to_string(), 
                    PolymorphicOctoPrintEventRequest::OctoPrintServerStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintServerStatusRequest{
                        payload: payload,
                        pi: pi_id,
                        event_type: *event_type,
                        octoprint_server
                    })
                );
                self.publish_octoprint_event(&subject, &payload).await

            }
            // pi.{pi_id}.octoprint.printer
            (subjects::SUBJECT_OCTOPRINT_PRINTER_STATUS, subargs) => {
                let payload = subargs.get_one::<String>("payload");
                let payload = match payload {
                    Some(data) => Some(serde_json::from_str::<HashMap<String, serde_json::Value>>(data)?),
                    None => None
                };
                let octoprint_server = self
                    .config
                    .octoprint
                    .as_ref()
                    .expect("Failed to readPrintNannyConfig.octoprint")
                    .server
                    .as_ref()
                    .expect("Failed to read PrintNannyConfig.octoprint.server")
                    .id;
                let event_type = self
                    .args
                    .get_one::<models::OctoPrintPrinterStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    stringify!(subjects::SUBJECT_OCTOPRINT_PRINTER_STATUS, pi_id = pi_id).to_string(), 
                    PolymorphicOctoPrintEventRequest::OctoPrintPrinterStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintPrinterStatusRequest{
                        payload: payload,
                        pi: pi_id,
                        event_type: *event_type,
                        octoprint_server
                    })
                );
                self.publish_octoprint_event(&subject, &payload).await

            }
            // end octoprint subject handlers

            // begin repetier subject handlers
            (subjects::SUBJECT_REPETIER, _) => unimplemented!("Publisher not implemented for {}", subjects::SUBJECT_REPETIER),
            // end repetier subject handlers
            // begin moonraker subject handlers
            (subjects::SUBJECT_MOONRAKER, _) => unimplemented!("Publisher not implemented for {}", subjects::SUBJECT_MOONRAKER),
            // end moonraker subject handlers
            _ => panic!("Invalid topic: {}", &topic),
        }

    }
}

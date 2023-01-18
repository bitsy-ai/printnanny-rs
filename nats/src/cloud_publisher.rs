use std::collections::HashMap;
use std::time::SystemTime;

use anyhow::Result;
use chrono::prelude::{DateTime, Utc};
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command, ValueEnum};
use futures::prelude::*;
use log::debug;

use printnanny_api_client::models;
use printnanny_api_client::models::polymorphic_octo_print_event_request::PolymorphicOctoPrintEventRequest;
use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;

use printnanny_edge_db::cloud::Pi;
use printnanny_edge_db::octoprint::OctoPrintServer;

use printnanny_services::error::ServiceError;
use printnanny_settings::printnanny::PrintNannySettings;

use tokio::net::UnixStream;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};
use uuid::Uuid;

use printnanny_services::error;

use crate::subjects;

pub const DEFAULT_NATS_CLOUD_PUBLISHER_APP_NAME: &str = "nats-cloud-publisher";

#[derive(Debug, Clone)]
pub struct CloudEventPublisher {
    args: ArgMatches,
    settings: PrintNannySettings,
    pi_id: i32,
    octoprint_server_id: i32,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum PayloadFormat {
    Json,
    // TODO: serialize raw
    // Bytes,
}

impl CloudEventPublisher {
    // initialize CloudEventPublisher from clap::Command ArgMatches
    pub fn new(args: &ArgMatches) -> Result<Self, ServiceError> {
        let settings = PrintNannySettings::new().unwrap();
        let pi_id = Pi::get_id()?;
        let octoprint_server_id = OctoPrintServer::get_id()?;
        Ok(Self {
            args: args.clone(),
            settings,
            pi_id,
            octoprint_server_id,
        })
    }
    pub fn clap_command(app_name: Option<String>) -> Command<'static> {
        let app_name =
            app_name.unwrap_or_else(|| DEFAULT_NATS_CLOUD_PUBLISHER_APP_NAME.to_string());

        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Issue command via NATs")
            .arg_required_else_help(true)
            .subcommand_required(true)
            .arg(Arg::new("subject").long("subject"))
            // begin octoprint topics
            .subcommand(
                Command::new(subjects::SUBJECT_OCTOPRINT_SERVER)
                    .arg(
                        Arg::new("event_type")
                            .long("event-type")
                            .takes_value(true)
                            .value_parser(value_parser!(models::OctoPrintServerStatusType)),
                    )
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .takes_value(true)
                            .value_parser(value_parser!(PayloadFormat))
                            .default_value("json")
                            .help("Payload format"),
                    )
                    .arg(
                        Arg::new("payload")
                            .long("payload")
                            .takes_value(true)
                            .help("UTF-8 encoded JSON payload"),
                    ),
            )
            .subcommand(
                Command::new(subjects::SUBJECT_OCTOPRINT_PRINTER_STATUS)
                    .arg(
                        Arg::new("event_type")
                            .long("event-type")
                            .takes_value(true)
                            .value_parser(value_parser!(models::OctoPrintPrinterStatusType)),
                    )
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .takes_value(true)
                            .value_parser(value_parser!(PayloadFormat))
                            .default_value("json")
                            .help("Payload format"),
                    )
                    .arg(
                        Arg::new("payload")
                            .long("payload")
                            .takes_value(true)
                            .help("UTF-8 encoded JSON payload"),
                    ),
            )
            .subcommand(
                Command::new(subjects::SUBJECT_OCTOPRINT_PRINT_JOB)
                    .arg(
                        Arg::new("event_type")
                            .long("event-type")
                            .takes_value(true)
                            .value_parser(value_parser!(models::OctoPrintPrintJobStatusType)),
                    )
                    .arg(
                        Arg::new("format")
                            .short('f')
                            .long("format")
                            .takes_value(true)
                            .value_parser(value_parser!(PayloadFormat))
                            .default_value("json")
                            .help("Payload format"),
                    )
                    .arg(
                        Arg::new("payload")
                            .long("payload")
                            .takes_value(true)
                            .help("UTF-8 encoded JSON payload"),
                    ),
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
                    .arg(
                        Arg::new("payload")
                            .long("payload")
                            .takes_value(true)
                            .help("UTF-8 encoded JSON payload"),
                    ),
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
                    .arg(
                        Arg::new("payload")
                            .long("payload")
                            .takes_value(true)
                            .help("UTF-8 encoded JSON payload"),
                    ),
            )
            // end moonraker topics
            // begin PrintNanny Pi topics
            .subcommand(
                Command::new(subjects::SUBJECT_COMMAND_BOOT).arg(
                    Arg::new("event_type")
                        .long("event-type")
                        .takes_value(true)
                        .value_parser(value_parser!(models::PiBootCommandType)),
                ),
            )
            .subcommand(
                Command::new(subjects::SUBJECT_STATUS_BOOT).arg(
                    Arg::new("event_type")
                        .long("event-type")
                        .takes_value(true)
                        .value_parser(value_parser!(models::PiBootStatusType)),
                ),
            )
            .subcommand(
                Command::new(subjects::SUBJECT_COMMAND_CAM).arg(
                    Arg::new("event_type")
                        .long("event-type")
                        .takes_value(true)
                        .value_parser(value_parser!(models::PiCamCommandType)),
                ),
            )
            .subcommand(
                Command::new(subjects::SUBJECT_STATUS_CAM).arg(
                    Arg::new("event_type")
                        .long("event-type")
                        .takes_value(true)
                        .value_parser(value_parser!(models::PiCamStatusType)),
                ),
            )
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
    pub async fn publish_pi_event(
        &self,
        subject: &str,
        payload: &PolymorphicPiEventRequest,
    ) -> Result<()> {
        let socket = &self.settings.paths.events_socket();
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
    pub async fn publish_octoprint_event(
        &self,
        subject: &str,
        payload: &PolymorphicOctoPrintEventRequest,
    ) -> Result<()> {
        let socket = &self.settings.paths.events_socket();
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
    fn socket_ok(&self) -> Result<(), error::NatsError> {
        let socket = &self.settings.paths.events_socket();
        match socket.exists() {
            true => Ok(()),
            false => Err(error::NatsError::UnixSocketNotFound {
                path: socket.display().to_string(),
            }),
        }
    }

    pub async fn run(self) -> Result<()> {
        self.socket_ok()?;

        let pi_id = self.pi_id;
        let id = Some(Uuid::new_v4().to_string());
        let created_dt: DateTime<Utc> = SystemTime::now().into();
        let created_dt = Some(created_dt.to_rfc3339());

        match self.args.subcommand().unwrap() {
            (subjects::SUBJECT_COMMAND_BOOT, subargs) => {
                let event_type = subargs
                    .get_one::<models::PiBootCommandType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    format!("pi.{pi_id}.command.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiBootCommandRequest(
                        models::polymorphic_pi_event_request::PiBootCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                            id,
                            created_dt,
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
                    format!("pi.{pi_id}.command.cam", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiCamCommandRequest(
                        models::polymorphic_pi_event_request::PiCamCommandRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                            id,
                            created_dt,
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
                    format!("pi.{pi_id}.command.swupdate", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiSoftwareUpdateCommandRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateCommandRequest {
                            version,
                            event_type: *event_type,
                            pi: pi_id,
                            payload: Box::new(payload),
                            id,
                            created_dt,
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
                    format!("pi.{pi_id}.status.boot", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiBootStatusRequest(
                        models::polymorphic_pi_event_request::PiBootStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                            id,
                            created_dt,
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
                    format!("pi.{pi_id}.status.cam", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiCamStatusRequest(
                        models::polymorphic_pi_event_request::PiCamStatusRequest {
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                            id,
                            created_dt,
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
                    format!("pi.{pi_id}.status.swupdate", pi_id = pi_id),
                    PolymorphicPiEventRequest::PiSoftwareUpdateStatusRequest(
                        models::polymorphic_pi_event_request::PiSoftwareUpdateStatusRequest {
                            version: version.to_string(),
                            event_type: *event_type,
                            pi: pi_id,
                            payload: None,
                            id,
                            created_dt,
                        },
                    ),
                );
                self.publish_pi_event(&subject, &payload).await
            }
            // pi.{pi_id}.octoprint.client
            (subjects::SUBJECT_OCTOPRINT_PRINT_JOB, subargs) => {
                let payload = subargs
                    .get_one::<String>("payload")
                    .expect("--payload is required");
                let payload =
                    serde_json::from_str::<models::OctoPrintPrintJobPayloadRequest>(payload)?;

                let octoprint_server = self.octoprint_server_id;
                let event_type = self
                    .args
                    .get_one::<models::OctoPrintPrintJobStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    format!("pi.{pi_id}.octoprint.print_job", pi_id = pi_id), 
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
                    Some(data) => Some(serde_json::from_str::<HashMap<String, serde_json::Value>>(
                        data,
                    )?),
                    None => None,
                };
                let octoprint_server = self.octoprint_server_id;

                let event_type = self
                    .args
                    .get_one::<models::OctoPrintServerStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    format!("pi.{pi_id}.octoprint.server", pi_id = pi_id), 
                    PolymorphicOctoPrintEventRequest::OctoPrintServerStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintServerStatusRequest{
                        payload,
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
                    Some(data) => Some(serde_json::from_str::<HashMap<String, serde_json::Value>>(
                        data,
                    )?),
                    None => None,
                };
                let octoprint_server = self.octoprint_server_id;

                let event_type = self
                    .args
                    .get_one::<models::OctoPrintPrinterStatusType>("event_type")
                    .expect("Invalid event_type");
                let (subject, payload) = (
                    format!("pi.{pi_id}.octoprint.printer", pi_id = pi_id), 
                    PolymorphicOctoPrintEventRequest::OctoPrintPrinterStatusRequest(
                        models::polymorphic_octo_print_event_request::OctoPrintPrinterStatusRequest{
                        payload,
                        pi: pi_id,
                        event_type: *event_type,
                        octoprint_server
                    })
                );
                self.publish_octoprint_event(&subject, &payload).await
            }
            // end octoprint subject handlers

            // begin repetier subject handlers
            (subjects::SUBJECT_REPETIER, _) => unimplemented!(
                "Publisher not implemented for {}",
                subjects::SUBJECT_REPETIER
            ),
            // end repetier subject handlers
            // begin moonraker subject handlers
            (subjects::SUBJECT_MOONRAKER, _) => unimplemented!(
                "Publisher not implemented for {}",
                subjects::SUBJECT_MOONRAKER
            ),
            // end moonraker subject handlers
            _ => panic!("Invalid subcommand {:?}", self.args.subcommand()),
        }
    }
}

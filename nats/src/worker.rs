use futures::prelude::*;
use std::path::PathBuf;

use anyhow::Result;
use bytes::Buf;
use clap::{crate_authors, Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{debug, error, info, warn, LevelFilter};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use printnanny_api_client::models;
use printnanny_services::config::{NatsConfig, PrintNannyConfig};
use printnanny_services::error::PrintNannyConfigError;

use crate::commands;
use crate::nats::NatsJsonEvent;

#[derive(Debug, Clone)]
pub struct Worker {
    socket: PathBuf,
    nats_client: async_nats::Client,
    subscribe_subject: String,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl Worker {
    pub async fn subscribe_nats_subject(&self) -> Result<()> {
        info!(
            "Subscribing to subect {} with nats client {:?}",
            self.subscribe_subject, &self.nats_client
        );
        let mut subscriber = self
            .nats_client
            .subscribe(self.subscribe_subject.clone())
            .await
            .unwrap();
        while let Some(message) = subscriber.next().await {
            debug!("Received NATS Message: {:?}", message);
            // try deserializing payload
            let payload: models::PolymorphicPiEvent =
                serde_json::from_reader(message.payload.reader())?;
            debug!("Deserialized PolymorphicPiEvent: {:?}", payload);
            commands::handle_incoming(payload).await?;
        }
        Ok(())
    }
    pub async fn relay_to_nats(&self, mut stream: UnixStream) -> Result<()> {
        debug!("Accepted socket connection {:?}", &stream);
        // read length-delimited JSON frames deserializable into NatsJsonEvent
        let length_delimited = FramedRead::new(&mut stream, LengthDelimitedCodec::new());
        let mut deserialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<NatsJsonEvent>::default(),
        );
        let maybe_msg: Option<NatsJsonEvent> = deserialized.try_next().await?;

        match maybe_msg {
            Some(msg) => {
                debug!("Deserialized NatsJsonEvent {:?}", msg);
                // publish over NATS connection
                let payload = serde_json::ser::to_vec(&msg.payload)?;
                self.nats_client
                    .publish(msg.subject, payload.into())
                    .await?;
            }
            None => error!("Failed to deserialize msg {:?}", maybe_msg),
        };
        Ok(())
    }

    pub async fn subscribe_event_socket(&self) -> Result<()> {
        let maybe_delete = std::fs::remove_file(&self.socket);
        match maybe_delete {
            Ok(_) => {
                warn!(
                    "Deleted socket {:?} without mercy. Refactor this code to run 2+ concurrent socket listeners/bindings.",
                    &self.socket
                );
            }
            Err(_) => {}
        };
        let listener = UnixListener::bind(&self.socket)?;
        info!("Listening for events on {:?}", self.socket);
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => self.relay_to_nats(stream).await?,
                Err(e) => {
                    error!("Connection to {} broken {}", &self.socket.display(), e);
                }
            }
        }
    }

    pub fn clap_command() -> Command<'static> {
        let app_name = "printnanny-events-worker";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Relay Unix socket data frames to outbound NATS connection. Handle inbound NATS msgs.")
            .arg(
                Arg::new("v")
                    .short('v')
                    .multiple_occurrences(true)
                    .help("Sets the level of verbosity"),
            );
        app
    }

    pub async fn new(args: ArgMatches) -> Result<Self> {
        let config = PrintNannyConfig::new()?;

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
        let subscribe_subject = match config.pi {
            Some(pi) => Ok(format!("pi.{}.*.command", pi.id)),
            None => Err(PrintNannyConfigError::LicenseMissing {
                path: "pi".to_string(),
            }),
        }?;
        // initialize nats connection
        let nats_client =
            async_nats::ConnectOptions::with_credentials_file(config.paths.nats_creds().clone())
                .await?
                .require_tls(config.nats.require_tls)
                .connect(config.nats.uri)
                .await?;
        return Ok(Self {
            socket: config.paths.events_socket.clone(),
            nats_client: nats_client,
            subscribe_subject,
        });
    }

    pub async fn run(&self) -> Result<()> {
        tokio::join!(self.subscribe_event_socket(), self.subscribe_nats_subject());
        Ok(())
    }
}

use anyhow::Result;
use bytes::Buf;
use clap::{crate_authors, ArgMatches, Command};
use futures::prelude::*;
use log::{debug, error, info, warn};
use std::io::Read;
use std::path::PathBuf;
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;
use printnanny_services::config::PrintNannyConfig;

// use crate::commands;
use crate::commands;
use crate::util::to_nats_command_subscribe_subject;

#[derive(Debug, Clone)]
pub struct NatsWorker {
    socket: PathBuf,
    nats_client: async_nats::Client,
    subscribe_subject: String,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl NatsWorker {
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
            let mut s = String::new();
            debug!("init String");
            message.payload.reader().read_to_string(&mut s)?;
            debug!("read message.payload to String");
            let payload = serde_json::from_str(&s);
            match payload {
                Ok(event) => {
                    debug!("Deserialized PolymorphicPiEvent: {:?}", event);
                    commands::handle_incoming(event, &self.nats_client).await?;
                }
                Err(e) => {
                    error!(
                        "Failed to deserialize PolymorphicPiEventRequest from {} with error {}",
                        &s, e
                    );
                }
            };
        }
        Ok(())
    }
    pub async fn relay_to_nats(&self, mut stream: UnixStream) -> Result<()> {
        debug!("Accepted socket connection {:?}", &stream);
        // read length-delimited JSON frames deserializable into NatsJsonEvent
        let length_delimited = FramedRead::new(&mut stream, LengthDelimitedCodec::new());
        let mut deserialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<(String, PolymorphicPiEventRequest)>::default(),
        );
        let maybe_msg: Option<(String, PolymorphicPiEventRequest)> =
            deserialized.try_next().await?;

        match maybe_msg {
            Some((subject, msg)) => {
                debug!("Deserialized {:?}", msg);
                // publish over NATS connection
                let payload = serde_json::ser::to_vec(&msg)?;
                self.nats_client.publish(subject, payload.into()).await?;
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
            Err(e) => {
                warn!(
                    "std::fs::remove_file({}) failed with error {:?}",
                    &self.socket.display(),
                    e
                )
            }
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
        let app_name = "nats-worker";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Run NATS-based pub/sub workers");
        app
    }

    pub async fn new(_args: &ArgMatches) -> Result<Self> {
        let config = PrintNannyConfig::new()?;
        // ensure pi, nats_app, nats_creds are provided
        config.try_check_license()?;

        // try_check_license guards the following properties set, so it's safe to unwrap here
        let pi = config.pi.unwrap();
        let nats_app = pi.nats_app.unwrap();

        let subscribe_subject = to_nats_command_subscribe_subject(&pi.id);

        // check if uri requires tls
        let require_tls = nats_app.nats_server_uri.contains("tls");

        // if nats.creds available, initialize authenticated nats connection
        let nats_client = match config.paths.nats_creds().exists() {
            true => {
                let credentials_file = config.paths.nats_creds().clone();
                async_nats::ConnectOptions::with_credentials_file(credentials_file)
                    .await?
                    .require_tls(require_tls)
                    .connect(nats_app.nats_server_uri)
                    .await?
            }
            false => {
                warn!(
                    "Failed to read {}. Initializing NATS client without credentials",
                    config.paths.nats_creds().display()
                );
                async_nats::ConnectOptions::new()
                    .require_tls(require_tls)
                    .connect(nats_app.nats_server_uri)
                    .await?
            }
        };

        return Ok(Self {
            socket: config.paths.events_socket.clone(),
            nats_client: nats_client,
            subscribe_subject,
        });
    }

    pub async fn run(&self) -> Result<()> {
        let (socket_task, nats_sub_task) =
            tokio::join!(self.subscribe_event_socket(), self.subscribe_nats_subject());
        socket_task?;
        nats_sub_task?;
        Ok(())
    }
}

use anyhow::Result;
use bytes::Buf;
use clap::{crate_authors, ArgMatches, Command};
use futures::prelude::*;
use log::{debug, error, info, warn};
use std::io::Read;
use std::path::PathBuf;
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{sleep, Duration};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

use printnanny_api_client::models::polymorphic_pi_event_request::PolymorphicPiEventRequest;
use printnanny_services::error::NatsError;

use printnanny_edge_db::nats_app::NatsApp;

use printnanny_settings::printnanny::PrintNannySettings;

use crate::cloud_commands;
use crate::util::to_nats_command_subscribe_subject;

pub const DEFAULT_NATS_CLOUD_APP_NAME: &str = "nats-cloud-worker";

#[derive(Debug, Clone)]
pub struct NatsCloudWorker {
    socket: PathBuf,
    subscribe_subject: String,
    nats_server_uri: String,
    require_tls: bool,
    nats_creds: PathBuf,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl NatsCloudWorker {
    pub async fn subscribe_nats_subject(&self) -> Result<()> {
        let mut nats_client: Option<async_nats::Client> = None;
        while nats_client.is_none() {
            match self.try_init_nats_client().await {
                Ok(nc) => {
                    nats_client = Some(nc);
                }
                Err(_) => {
                    warn!("Waiting for NATS client to initialize subscriber thread");
                    sleep(Duration::from_millis(2000)).await;
                }
            }
        }
        warn!(
            "Subscribing to subect {} with nats client {:?}",
            self.subscribe_subject, nats_client
        );
        let nats_client = nats_client.unwrap();
        let mut subscriber = nats_client
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
                    cloud_commands::handle_incoming(event, message.reply, &nats_client).await?;
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

    pub async fn deserialize_socket_msg(
        &self,
        mut stream: UnixStream,
    ) -> Result<Option<(String, Vec<u8>)>> {
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
                debug!(
                    "Published on subject={} server={}",
                    &subject, &self.nats_server_uri
                );
                Ok(Some((subject, payload)))
            }
            None => {
                error!("Failed to deserialize msg {:?}", maybe_msg);
                Ok(None)
            }
        }
    }

    // FIFO buffer flush
    pub async fn try_flush_buffer(
        &self,
        event_buffer: &[(String, Vec<u8>)],
        nats_client: &async_nats::Client,
    ) -> Result<(), NatsError> {
        for event in event_buffer.iter() {
            let (subject, payload) = event;
            match nats_client
                .publish(subject.to_string(), payload.clone().into())
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(NatsError::PublishError {
                    error: e.to_string(),
                }),
            }?;
        }

        Ok(())
    }

    pub async fn try_init_nats_client(&self) -> Result<async_nats::Client, std::io::Error> {
        match self.nats_creds.exists() {
            true => {
                async_nats::ConnectOptions::with_credentials_file(self.nats_creds.clone())
                    .await?
                    .require_tls(self.require_tls)
                    .connect(&self.nats_server_uri)
                    .await
            }
            false => {
                warn!(
                    "Failed to read {}. Initializing NATS client without credentials",
                    self.nats_creds.display()
                );
                async_nats::ConnectOptions::new()
                    .require_tls(self.require_tls)
                    .connect(&self.nats_server_uri)
                    .await
            }
        }
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
        let mut nats_client: Option<async_nats::Client> = None;

        let max_buffer_size: usize = 12;

        let mut event_buffer: Vec<(String, Vec<u8>)> = vec![];
        loop {
            // is nats client connected?
            nats_client = match nats_client {
                Some(nc) => Some(nc),
                None => match self.try_init_nats_client().await {
                    Ok(nc) => Some(nc),
                    Err(_) => {
                        warn!("NATS client not yet initialized");
                        None
                    }
                },
            };

            event_buffer = match &nats_client {
                Some(nc) => {
                    self.try_flush_buffer(&event_buffer, nc).await?;
                    vec![]
                }
                None => event_buffer,
            };

            // add new message to queue
            match listener.accept().await {
                Ok((stream, _addr)) => match self.deserialize_socket_msg(stream).await {
                    Ok(msg) => {
                        if msg.is_some() {
                            // if buffer is full, drop head event
                            if event_buffer.len() >= max_buffer_size {
                                if let Some((head, rest)) = event_buffer.split_first() {
                                    let (subject, payload) = head;
                                    let payload: PolymorphicPiEventRequest =
                                        serde_json::from_slice(payload)?;
                                    warn!("Event buffer is full (max size: {}). Dropping oldest event on subject={} payload={:?}", max_buffer_size, subject, payload);
                                    event_buffer = rest.to_vec();
                                }
                            }

                            event_buffer.push(msg.unwrap());
                        }
                    }
                    Err(e) => error!("Error relaying to NATS {:?}", e),
                },
                Err(e) => {
                    error!("Connection to {} broken {}", &self.socket.display(), e);
                }
            }
        }
    }

    pub fn clap_command(app_name: Option<String>) -> Command<'static> {
        let app_name = app_name.unwrap_or_else(|| DEFAULT_NATS_CLOUD_APP_NAME.to_string());
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Run NATS-based pub/sub workers");
        app
    }

    pub async fn new(_args: &ArgMatches) -> Result<Self> {
        let config = PrintNannySettings::new().await?;
        let sqlite_connection = config.paths.db().display().to_string();

        let nats_app = NatsApp::get(&sqlite_connection)?;

        // try_check_license guards the following properties set, so it's safe to unwrap here
        let subscribe_subject = to_nats_command_subscribe_subject(&nats_app.pi_id);

        // check if uri requires tls
        let require_tls = nats_app.nats_server_uri.contains("tls");

        // if nats.creds available, initialize authenticated nats connection
        info!(
            "Attempting to initialize NATS connection to {}",
            nats_app.nats_server_uri
        );
        let nats_creds = config.paths.cloud_nats_creds();

        Ok(Self {
            socket: config.paths.events_socket(),
            subscribe_subject,
            nats_server_uri: nats_app.nats_server_uri,
            nats_creds,
            require_tls,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let (socket_task, nats_sub_task) =
            tokio::join!(self.subscribe_event_socket(), self.subscribe_nats_subject());
        socket_task?;
        nats_sub_task?;
        Ok(())
    }
}

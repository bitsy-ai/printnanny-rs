use futures::prelude::*;
use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{debug, error, info, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NatsConfig {
    uri: String,
    require_tls: bool,
    creds_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NatsJsonEvent {
    subject: String,
    payload: serde_json::value::Value,
}

#[derive(Debug, Clone)]
pub struct Worker {
    nats: NatsConfig,
    socket: PathBuf,
}

// Relays NatsJsonEvent published to Unix socket to NATS
impl Worker {
    pub async fn relay_to_nats(
        mut stream: UnixStream,
        nats_client: &async_nats::Client,
    ) -> Result<()> {
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
                nats_client.publish(msg.subject, payload.into()).await?;
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

        // initialize nats connection
        let nats_client = match &self.nats.creds_file {
            Some(creds_file) => {
                async_nats::ConnectOptions::with_credentials_file(creds_file.to_path_buf())
                    .await?
                    .require_tls(self.nats.require_tls)
                    .connect(&self.nats.uri)
                    .await?
            }
            None => async_nats::connect(&self.nats.uri).await?,
        };
        info!("Initialized nats client {:?}", nats_client);
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => Worker::relay_to_nats(stream, &nats_client).await?,
                Err(e) => {
                    error!("Connection to {} broken {}", &self.socket.display(), e);
                }
            }
        }
    }

    pub fn clap_command() -> Command<'static> {
        let app_name = "printnanny-pub";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Relay Unix socket data frames to NATS connection")
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
                    .help("Path to Unix socket"),
            )
            .arg(
                Arg::new("nats_uri")
                    .long("nats-uri")
                    .default_value("nats://localhost:4222")
                    .takes_value(true)
                    .help("NATS connection uri"),
            )
            .arg(
                Arg::new("nats_creds")
                    .long("nats-creds")
                    .takes_value(true)
                    .help("Path to nkey.creds file"),
            )
            .arg(
                Arg::new("nats_tls")
                    .long("nats-tls")
                    .help("Connect with tls"),
            );
        app
    }

    pub fn new(args: ArgMatches) -> Self {
        let socket = args
            .value_of("socket")
            .expect("--socket is required")
            .into();
        let uri = args
            .value_of("nats_uri")
            .expect("--nats-uri is required")
            .into();
        let creds_file: Option<PathBuf> = args.value_of("nats_creds").map(|v| PathBuf::from(v));
        let require_tls = args.is_present("nats_tls");
        let nats = NatsConfig {
            uri,
            creds_file,
            require_tls,
        };
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
        return Self { nats, socket };
    }

    pub async fn run(&self) -> Result<()> {
        self.subscribe_event_socket().await
    }
}

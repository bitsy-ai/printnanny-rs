use futures::prelude::*;

use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{info, LevelFilter};

use super::nats::NatsConfig;

#[derive(Debug, Clone)]
pub struct Worker {
    nats: NatsConfig,
    subject: String,
}

// Subscribes to NATS command subject
impl Worker {
    pub async fn subscribe_nats_subject(&self) -> Result<()> {
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
        info!(
            "Subscribing to subect {} with nats client {:?}",
            self.subject, &nats_client
        );
        let mut subscriber = nats_client.subscribe(self.subject.clone()).await.unwrap();
        while let Some(message) = subscriber.next().await {
            println!("Received message {:?}", message);
        }
        Ok(())
    }
    pub fn clap_command() -> Command<'static> {
        let app_name = "printnanny-sub";
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
                Arg::new("subject")
                    .long("subject")
                    .takes_value(true)
                    .help("Subscribe to subject/pattern"),
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
        let subject = args
            .value_of("subject")
            .expect("--subject is required")
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
        return Self { nats, subject };
    }

    pub async fn run(&self) -> Result<()> {
        self.subscribe_nats_subject().await
    }
}

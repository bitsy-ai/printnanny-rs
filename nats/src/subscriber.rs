use std::fmt::Debug;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;

use bytes::Buf;
use clap::{crate_authors, Arg, ArgMatches, Command};
use futures::stream::StreamExt;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

use super::error::{CommandError, NatsError};
use super::message::{MessageHandler, MessageResponse, NatsQcCommandRequest, ResponseStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsSubscriber<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug + MessageHandler,
    Response: Serialize + DeserializeOwned + Debug + MessageResponse<Request, Response>,
{
    subject: String,
    nats_server_uri: String,
    require_tls: bool,
    nats_creds: Option<PathBuf>,
    _request: PhantomData<Request>,
    _response: PhantomData<Response>,
}

const DEFAULT_NATS_SOCKET_PATH: &str = "/var/run/printnanny/nats-worker.sock";
const DEFAULT_NATS_URI: &str = "nats://localhost:4222";
const DEFAULT_NATS_SUBJECT: &str = "pi.*";

impl<Request, Response> NatsSubscriber<Request, Response>
where
    Request: Serialize + DeserializeOwned + Debug + MessageHandler,
    Response: Serialize + DeserializeOwned + Debug + MessageResponse<Request, Response>,
{
    pub fn clap_command() -> Command<'static> {
        let app_name = "nats-edge-worker";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Run NATS-based pub/sub workers")
            .arg(
                Arg::new("subject")
                    .long("subject")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_SUBJECT),
            )
            .arg(
                Arg::new("nats_server_uri")
                    .long("nats-server-uri")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_URI),
            )
            .arg(Arg::new("nats_creds").long("nats-creds").takes_value(true))
            .arg(
                Arg::new("socket")
                    .long("socket")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_SOCKET_PATH),
            );
        app
    }

    pub fn new(args: &ArgMatches) -> Self {
        let subject = args
            .value_of("subject")
            .unwrap_or_else(|| DEFAULT_NATS_SUBJECT);

        // check if uri requires tls
        let nats_server_uri: &str = args
            .value_of("nats_server_uri")
            .unwrap_or_else(|| DEFAULT_NATS_URI);
        let require_tls = nats_server_uri.contains("tls");

        // if nats.creds available, initialize authenticated nats connection
        info!(
            "Attempting to initialize NATS connection to {}",
            nats_server_uri
        );

        let nats_creds = args.value_of("nats_creds");
        let nats_creds = nats_creds.map(|v| PathBuf::from(v));

        Self {
            subject: subject.to_string(),
            nats_server_uri: nats_server_uri.to_string(),
            nats_creds,
            require_tls,
            _request: PhantomData,
            _response: PhantomData,
        }
    }

    pub async fn subscribe_nats_subject(&self) -> Result<(), CommandError> {
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
            self.subject, nats_client
        );
        let nats_client = nats_client.unwrap();
        let mut subscriber = nats_client.subscribe(self.subject.clone()).await.unwrap();
        while let Some(message) = subscriber.next().await {
            debug!("Received NATS Message: {:?}", message);
            // try deserializing payload
            let mut s = String::new();
            debug!("init String");
            message.payload.reader().read_to_string(&mut s)?;
            debug!("read message.payload to String");
            let payload = serde_json::from_str::<Request>(&s);
            let res: Response = match payload {
                Ok(event) => {
                    info!("Deserialized request: {:?}", event);
                    event.handle()?;
                    Response::new(Some(event), ResponseStatus::Ok, "".into())
                }
                Err(e) => {
                    let detail = format!("Failed to deserialize {} with error {}", &s, e);
                    error!("{}", &detail);
                    let err = CommandError::SerdeJson {
                        payload: s.to_string(),
                        error: e.to_string(),
                        source: e,
                    };
                    Response::new(None, ResponseStatus::Error, detail)
                }
            };
            match message.reply {
                Some(reply_inbox) => {
                    let payload = serde_json::to_vec(&res).unwrap();
                    match nats_client.publish(reply_inbox, payload.into()).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(CommandError::NatsError(NatsError::PublishError {
                            error: e.to_string(),
                        })),
                    }
                }
                None => Ok(()),
            }?;
        }
        Ok(())
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
        match &self.nats_creds {
            Some(nats_creds) => match nats_creds.exists() {
                true => {
                    async_nats::ConnectOptions::with_credentials_file(nats_creds.clone())
                        .await?
                        .require_tls(self.require_tls)
                        .connect(&self.nats_server_uri)
                        .await
                }
                false => {
                    warn!(
                        "Failed to read {}. Initializing NATS client without credentials",
                        nats_creds.display()
                    );
                    async_nats::ConnectOptions::new()
                        .require_tls(self.require_tls)
                        .connect(&self.nats_server_uri)
                        .await
                }
            },
            None => {
                async_nats::ConnectOptions::new()
                    .require_tls(self.require_tls)
                    .connect(&self.nats_server_uri)
                    .await
            }
        }
    }
    pub async fn run(&self) -> Result<(), CommandError> {
        self.subscribe_nats_subject().await?;
        Ok(())
    }
}

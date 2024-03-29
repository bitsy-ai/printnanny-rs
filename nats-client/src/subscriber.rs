use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command};
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use printnanny_settings::sys_info;

use super::client::wait_for_nats_client;
use super::event::NatsEventHandler;
use super::request_reply::NatsRequestHandler;
use crate::error::{NatsError, RequestErrorMsg};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsSubscriber<Event, Request, Reply>
where
    Event: Serialize + DeserializeOwned + Debug + NatsEventHandler,
    Request: Serialize + DeserializeOwned + Debug + NatsRequestHandler,
    Reply: Serialize + DeserializeOwned + Debug,
{
    subject: String,
    nats_server_uri: String,
    hostname: String,
    require_tls: bool,
    workers: usize,
    nats_creds: Option<PathBuf>,
    _event: PhantomData<Event>,
    _request: PhantomData<Request>,
    _response: PhantomData<Reply>,
}

const DEFAULT_NATS_SOCKET_PATH: &str = "/var/run/printnanny/nats-worker.sock";
const DEFAULT_NATS_URI: &str = "nats://localhost:4223";

pub const DEFAULT_NATS_EDGE_APP_NAME: &str = "nats-edge-worker";
pub const DEFAULT_NATS_EDGE_SUBJECT: &str = "pi.localhost.>";

pub fn get_default_nats_subject() -> String {
    let hostname = sys_info::hostname().unwrap();
    format!("pi.{}.>", hostname)
}

impl<Event, Request, Reply> NatsSubscriber<Event, Request, Reply>
where
    Event: Serialize + DeserializeOwned + Debug + NatsEventHandler<Event = Event>,
    Request: Serialize
        + DeserializeOwned
        + Debug
        + NatsRequestHandler<Request = Request>
        + NatsRequestHandler<Reply = Reply>
        + std::marker::Sync
        + std::marker::Send,
    Reply: Serialize + DeserializeOwned + Debug + std::marker::Sync,
{
    pub fn clap_command(app_name: Option<String>) -> Command<'static> {
        let app_name = app_name.unwrap_or_else(|| DEFAULT_NATS_EDGE_APP_NAME.to_string());

        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Run NATS-based pub/sub workers")
            .arg(
                Arg::new("v")
                    .short('v')
                    .multiple_occurrences(true)
                    .help("Sets the level of verbosity. Info: -v Debug: -vv Trace: -vvv"),
            )
            .arg(
                Arg::new("subject")
                    .long("subject")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_EDGE_SUBJECT),
            )
            .arg(
                Arg::new("nats_server_uri")
                    .long("nats-server-uri")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_URI),
            )
            .arg(Arg::new("hostname").long("hostname").takes_value(true))
            .arg(Arg::new("nats_creds").long("nats-creds").takes_value(true))
            .arg(
                Arg::new("workers")
                    .long("workers")
                    .takes_value(true)
                    .default_value("8"),
            )
            .arg(
                Arg::new("socket")
                    .long("socket")
                    .takes_value(true)
                    .default_value(DEFAULT_NATS_SOCKET_PATH),
            );
        app
    }

    pub fn new(args: &ArgMatches) -> Self {
        let default_nats_subject = get_default_nats_subject();

        let subject = args
            .value_of("subject")
            .unwrap_or(&default_nats_subject)
            .to_string()
            // always subscribe to lowercased hostname pattern
            // see https://github.com/bitsy-ai/printnanny-os/issues/238
            .to_lowercase();

        // check if uri requires tls
        let nats_server_uri: &str = args.value_of("nats_server_uri").unwrap_or(DEFAULT_NATS_URI);
        let require_tls = nats_server_uri.contains("tls");

        // if nats.creds available, initialize authenticated nats connection
        info!(
            "Attempting to initialize NATS connection to {}",
            nats_server_uri
        );

        let nats_creds = args.value_of("nats_creds");
        let nats_creds = nats_creds.map(PathBuf::from);

        let system_hostname = sys_info::hostname().unwrap_or_else(|_| "localhost".into());
        let hostname = args
            .value_of("hostname")
            .unwrap_or(&system_hostname)
            .to_string()
            // always subscribe to lowercased hostname pattern
            // see https://github.com/bitsy-ai/printnanny-os/issues/238
            .to_lowercase();
        let workers: usize = args.value_of_t("workers").unwrap_or(8);
        Self {
            hostname,
            subject,
            nats_server_uri: nats_server_uri.to_string(),
            nats_creds,
            require_tls,
            workers,
            _event: PhantomData,
            _request: PhantomData,
            _response: PhantomData,
        }
    }
    pub async fn subscribe_nats_subject(&self) -> Result<()> {
        let nats_client = wait_for_nats_client(
            &self.nats_server_uri,
            &self.nats_creds,
            self.require_tls,
            2000,
        )
        .await?;
        warn!(
            "Subscribing to subject {} with nats client {:?}",
            self.subject, nats_client
        );
        let subscriber = nats_client.subscribe(self.subject.clone()).await.unwrap();
        warn!(
            "Listening on {} where subject={}",
            &self.nats_server_uri, &self.subject
        );

        subscriber
            .for_each_concurrent(self.workers, |message| async {
                let subject_pattern =
                    Request::replace_subject_pattern(&message.subject, &self.hostname, "{pi_id}");
                debug!(
                    "Extracted subject_pattern {} from subject {} using hostname {}",
                    &subject_pattern, &message.subject, &self.hostname
                );
                debug!("Attempting to handle NATS Message: {:?}", message);
                match message.reply {
                    // request / reply pattern
                    Some(reply_inbox) => {
                        let payload = self
                            .handle_request(&message.payload, &subject_pattern)
                            .await;
                        match payload {
                            Some(payload) => {
                                match &nats_client.publish(reply_inbox, payload.into()).await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        error!("Error publishing msg: {}", e);
                                    }
                                }
                            }
                            None => {
                                warn!(
                                    "Expected reply payload for {}, but received None",
                                    &reply_inbox
                                )
                            }
                        }
                    }
                    // one-way event handler
                    None => {
                        self.handle_event(&message.payload, &subject_pattern).await;
                    }
                }
            })
            .await;
        Ok(())
    }
    // FIFO buffer flush
    pub async fn try_flush_buffer(
        &self,
        request_buffer: &[(String, Vec<u8>)],
        nats_client: &async_nats::Client,
    ) -> Result<(), NatsError> {
        for request in request_buffer.iter() {
            let (subject, payload) = request;
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

    async fn handle_request(
        &self,
        payload: &bytes::Bytes,
        subject_pattern: &str,
    ) -> Option<Vec<u8>> {
        match Request::deserialize_payload(subject_pattern, payload) {
            Ok(request) => match request.handle().await {
                Ok(r) => Some(serde_json::to_vec(&r).unwrap()),
                Err(e) => {
                    let r = RequestErrorMsg {
                        error: e.to_string(),
                        subject_pattern: subject_pattern.to_string(),
                        request,
                    };
                    Some(serde_json::to_vec(&r).unwrap())
                }
            },
            Err(e) => {
                error!("Error deserializing NATS request error={}", e);
                None
            }
        }
    }

    async fn handle_event(&self, payload: &bytes::Bytes, subject_pattern: &str) {
        match Event::deserialize_payload(subject_pattern, payload) {
            Ok(event) => match event.handle().await {
                Ok(_) => debug!("Success handling event={}", subject_pattern),
                Err(e) => error!("Error handling event={} error={}", subject_pattern, e),
            },
            Err(e) => {
                error!("Error deserializing NATS event error={}", e);
            }
        }
    }

    pub async fn run(&self) -> Result<()> {
        self.subscribe_nats_subject().await?;
        Ok(())
    }
}

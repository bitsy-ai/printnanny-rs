use std::path::PathBuf;

use clap::{crate_authors, Arg, ArgMatches, Command};
use log::info;

use super::error::NatsError;

#[derive(Debug, Clone)]
pub struct NatsWorker {
    socket: PathBuf,
    subscribe_subject: String,
    nats_server_uri: String,
    require_tls: bool,
    nats_creds: Option<PathBuf>,
}

const DEFAULT_NATS_SOCKET_PATH: &str = "/var/run/printnanny/nats-worker.sock";
const DEFAULT_NATS_URI: &str = "nats://localhost:4222";
const DEFAULT_NATS_SUBJECT: &str = "pi.*";

impl NatsWorker {
    type NatsEvent;
    fn clap_command() -> Command<'static> {
        let app_name = "nats-worker";
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

    async fn new(args: &ArgMatches) -> Self {
        let subject: &str = args
            .value_of_t("subject")
            .unwrap_or_else(|_| DEFAULT_NATS_SUBJECT);

        // check if uri requires tls
        let nats_server_uri: &str = args
            .value_of_t("nats_server_uri")
            .unwrap_or_else(|_| DEFAULT_NATS_URI);

        let require_tls = nats_server_uri.nats_server_uri.contains("tls");

        // if nats.creds available, initialize authenticated nats connection
        info!(
            "Attempting to initialize NATS connection to {}",
            nats_server_uri
        );

        let nats_creds = args.value("nats_creds");

        let socket = args
            .value_of_t("socket")
            .unwrap_or_else(|_| DEFAULT_NATS_SOCKET_PATH);

        Self {
            socket,
            subject,
            nats_server_uri,
            nats_creds,
            require_tls,
        }
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
            tokio_serde::formats::SymmetricalJson::<(String, NatsEvent)>::default(),
        );
        let maybe_msg: Option<(String, NatsEvent)> = deserialized.try_next().await?;

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

    pub async fn subscribe_nats_subject(&self) -> Result<(), NatsError> {
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
                    commands::handle_incoming(event, message.reply, &nats_client).await?;
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
        match self.nats_creds {
            Some(nats_creds) => match nats_creds.exists() {
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
            },
            None => {
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
                                match event_buffer.split_first() {
                                    Some((head, rest)) => {
                                        let (subject, payload) = head;
                                        let payload: NatsEvent = serde_json::from_slice(payload)?;
                                        warn!("Event buffer is full (max size: {}). Dropping oldest event on subject={} payload={:?}", max_buffer_size, subject, payload);
                                        event_buffer = rest.to_vec();
                                    }
                                    None => (),
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

    pub async fn run(&self) -> Result<()> {
        let (socket_task, nats_sub_task) =
            tokio::join!(self.subscribe_event_socket(), self.subscribe_nats_subject());
        socket_task?;
        nats_sub_task?;
        Ok(())
    }
}

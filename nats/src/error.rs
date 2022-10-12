use thiserror::Error;

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("Connection to {path} failed")]
    UnixSocketNotFound { path: String },
    #[error("NatsConnection error {msg}")]
    NatsConnection { msg: String },

    #[error("Nats PublishError {error}")]
    PublishError { error: String },
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Failed to parse key=value pair from systemctl output")]
    SystemctlParse { output: String },

    #[error("Failed to deserialize {payload} with error {error}")]
    SerdeJson {
        payload: String,
        error: String,
        source: serde_json::Error,
    },

    #[error(transparent)]
    NatsError(#[from] NatsError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

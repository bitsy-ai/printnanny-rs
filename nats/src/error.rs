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
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

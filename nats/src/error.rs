use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error("Connection to {path} failed")]
    UnixSocketNotFound { path: String },
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Failed to parse key=value pair from systemctl output")]
    SystemctlParse { output: String },
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

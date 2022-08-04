use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error("Connection to {path} failed")]
    UnixSocketNotFound { path: String },
}

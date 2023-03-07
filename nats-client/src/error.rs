use std::fmt;
use std::fmt::Debug;

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NatsError {
    #[error("Connection to {path} failed")]
    UnixSocketNotFound { path: String },
    #[error("NatsConnection error {msg}")]
    NatsConnection { msg: String },

    #[error("Nats PublishError {error}")]
    PublishError { error: String },

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

#[derive(Error, Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RequestErrorMsg<Request: Serialize + Debug> {
    pub subject_pattern: String,
    pub request: Request,
    pub error: String,
}

impl<Request: Serialize + Debug> fmt::Display for RequestErrorMsg<Request> {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "Error handling NatsRequest: {} Request: {:?}",
            self.error, self.request
        )
    }
}

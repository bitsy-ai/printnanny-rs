use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
pub struct ErrorMessage {
    pub src: String,
    pub error: String,
    pub debug: Option<String>,
}

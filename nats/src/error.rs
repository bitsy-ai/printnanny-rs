use std::fmt;
use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::message::{NatsReply, NatsRequest, NatsRequestReplyHandler};

#[derive(Error, Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RequestErrorMsg<Request: Serialize + Debug> {
    pub request: Request,
    pub msg: String,
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
            self.msg, self.request
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(tag = "status")]
pub enum ReplyResult<Request: Serialize + Debug, Response: Serialize + Debug> {
    #[serde(rename = "ok")]
    Ok(Response),
    #[serde(rename = "error")]
    Err(RequestErrorMsg<Request>),
}

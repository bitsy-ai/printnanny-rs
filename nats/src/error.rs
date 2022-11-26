use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Deserialize, Serialize)]
pub struct ErrorMsg<Request> {
    request: Request,
    msg: String,
}

pub type ResultMsg<T> = Result<T, ErrorMsg<T>>;

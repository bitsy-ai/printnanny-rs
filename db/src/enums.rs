use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use printnanny_api_client;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, DbEnum)]
pub enum RecordingStatus {
    Pending,
    InProgress,
    Done,
}

impl Default for RecordingStatus {
    fn default() -> Self {
        RecordingStatus::Pending
    }
}

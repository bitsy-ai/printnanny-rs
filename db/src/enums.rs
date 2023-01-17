use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, DbEnum)]
pub enum Status {
    Pending,
    InProgress,
    Done,
}

impl Default for Status {
    fn default() -> Self {
        Status::Pending
    }
}

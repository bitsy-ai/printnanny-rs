use printnanny_api_client::models;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OctoPrintConfig {
    pub server: Option<models::OctoPrintServer>,
}

impl Default for OctoPrintConfig {
    fn default() -> Self {
        Self { server: None }
    }
}

use anyhow::Result;
use clap::ArgEnum;
use log::info;
use printnanny_api_client::models;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JanusConfig {
    pub edge: models::JanusStream,
    pub cloud: models::JanusStream,
}

#[derive(PartialEq, Debug, Clone, Copy, ArgEnum)]
pub enum JanusAdminEndpoint {
    GetStatus,
    Info,
    Ping,
    AddToken,
    RemoveToken,
    ListTokens,
    TestStun,
}

impl JanusAdminEndpoint {
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        JanusAdminEndpoint::value_variants()
            .iter()
            .filter_map(clap::ArgEnum::to_possible_value)
    }
    pub fn to_action(&self) -> String {
        let action = match self {
            JanusAdminEndpoint::GetStatus => "get_status",
            JanusAdminEndpoint::Info => "info",
            JanusAdminEndpoint::Ping => "ping",
            JanusAdminEndpoint::AddToken => "add_token",
            JanusAdminEndpoint::RemoveToken => "remove_token",
            JanusAdminEndpoint::ListTokens => "list_tokens",
            JanusAdminEndpoint::TestStun => "test_stun",
        };
        action.to_string()
    }
}

impl std::str::FromStr for JanusAdminEndpoint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

#[derive(Debug, Clone)]
pub struct JanusAdminService {
    pub host: String,
    pub admin_secret: Option<String>,
    pub token: Option<String>,
}

fn _build_request_body(
    endpoint: &JanusAdminEndpoint,
    janus_config: &models::JanusStream,
) -> Result<HashMap<String, String>> {
    let transaction: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let action = endpoint.to_action();
    let mut map = HashMap::new();
    map.insert(String::from("transaction"), transaction);
    map.insert(String::from("janus"), action);
    info!("Loaded JanusEdgeConfig={:?}", janus_config);
    info!(
        "Building Janus Admin API {:?} request body {:?}",
        &endpoint, &map
    );
    match endpoint {
        JanusAdminEndpoint::AddToken => {
            unimplemented!("JanusAdminEndpoint::AddToken not implemented");
        }
        JanusAdminEndpoint::RemoveToken => {
            unimplemented!("JanusAdminEndpoint::RemoveToken not implemented");
        }
        JanusAdminEndpoint::ListTokens => {
            unimplemented!("JanusAdminEndpoint::ListTokens not implemented");
        }
        JanusAdminEndpoint::GetStatus => {
            unimplemented!("JanusAdminEndpoint::GetStatus not implemented");
        }
        JanusAdminEndpoint::TestStun => {
            unimplemented!("JanusAdminEndpoint::TestStun not implemented");
        }
        _ => {}
    };
    Ok(map)
}

pub async fn janus_admin_api_call(_endpoint: JanusAdminEndpoint) -> Result<String> {
    unimplemented!("janus_admin_api_call is not yet implemented")
    // let janus_config = PrintNannyConfig::new()?
    //     .janus_edge_stream
    //     .expect("janus_edge config is not set");
    // let body = build_request_body(&endpoint, &janus_config)?;
    // let client = reqwest::Client::new();
    // let host = &janus_config.admin_url;
    // let res = client.post(host).json(&body).send().await?.text().await?;
    // Ok(res)
}

impl JanusAdminService {
    pub fn new(
        host: String,
        admin_secret: Option<String>,
        token: Option<String>,
    ) -> JanusAdminService {
        Self {
            host,
            admin_secret,
            token,
        }
    }
}

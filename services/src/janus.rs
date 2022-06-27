use anyhow::Result;
use clap::ArgEnum;
use log::info;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

use printnanny_api_client::models;

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

fn build_request_body(
    endpoint: &JanusAdminEndpoint,
    janus_config: &models::JanusEdgeStream,
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
            map.insert(
                String::from("token"),
                janus_config
                    .auth
                    .as_ref()
                    .api_token
                    .as_ref()
                    .expect("api_token not set")
                    .clone(),
            );
            map.insert(
                String::from("admin_secret"),
                janus_config
                    .auth
                    .as_ref()
                    .admin_secret
                    .as_ref()
                    .expect("admin_secret not set")
                    .clone(),
            );
        }
        JanusAdminEndpoint::RemoveToken => {
            map.insert(
                String::from("token"),
                janus_config
                    .auth
                    .as_ref()
                    .api_token
                    .as_ref()
                    .expect("api_token not set")
                    .clone(),
            );
            map.insert(
                String::from("admin_secret"),
                janus_config
                    .auth
                    .as_ref()
                    .admin_secret
                    .as_ref()
                    .expect("admin_secret not set")
                    .clone(),
            );
        }
        JanusAdminEndpoint::ListTokens => {
            map.insert(
                String::from("admin_secret"),
                janus_config
                    .auth
                    .as_ref()
                    .admin_secret
                    .as_ref()
                    .expect("admin_secret not set")
                    .clone(),
            );
        }
        JanusAdminEndpoint::GetStatus => {
            map.insert(
                String::from("token"),
                janus_config
                    .auth
                    .as_ref()
                    .api_token
                    .as_ref()
                    .expect("api_token not set")
                    .clone(),
            );
            map.insert(
                String::from("admin_secret"),
                janus_config
                    .auth
                    .as_ref()
                    .admin_secret
                    .as_ref()
                    .expect("admin_secret not set")
                    .clone(),
            );
        }
        JanusAdminEndpoint::TestStun => {
            map.insert(
                String::from("admin_secret"),
                janus_config
                    .auth
                    .as_ref()
                    .admin_secret
                    .as_ref()
                    .expect("admin_secret not set")
                    .clone(),
            );
        }
        _ => {}
    };
    Ok(map)
}

pub async fn janus_admin_api_call(endpoint: JanusAdminEndpoint) -> Result<String> {
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

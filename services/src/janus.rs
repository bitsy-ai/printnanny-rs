use anyhow::{anyhow, Result};
use clap::ArgEnum;
use log::debug;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use reqwest;

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

fn validate_request_field(
    endpoint: &JanusAdminEndpoint,
    field: &str,
    value: Option<String>,
) -> Result<String> {
    match value {
        Some(t) => Ok(t),
        None => Err(anyhow!("{} is required by {:?}", field, endpoint)),
    }
}

fn build_request_body(
    endpoint: &JanusAdminEndpoint,
    token: Option<String>,
    admin_secret: Option<String>,
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
    debug!(
        "Building Janus Admin API {:?} request body {:?}",
        &endpoint, &map
    );
    match endpoint {
        JanusAdminEndpoint::AddToken => {
            map.insert(
                String::from("token"),
                validate_request_field(&endpoint, "token", token)?,
            );
            map.insert(
                String::from("admin_secret"),
                validate_request_field(&endpoint, "admin_secret", admin_secret)?,
            );
        }
        JanusAdminEndpoint::RemoveToken => {
            map.insert(
                String::from("token"),
                validate_request_field(&endpoint, "token", token)?,
            );
            map.insert(
                String::from("admin_secret"),
                validate_request_field(&endpoint, "admin_secret", admin_secret)?,
            );
        }
        JanusAdminEndpoint::ListTokens => {
            map.insert(
                String::from("admin_secret"),
                validate_request_field(&endpoint, "admin_secret", admin_secret)?,
            );
        }
        JanusAdminEndpoint::GetStatus => {
            map.insert(
                String::from("token"),
                validate_request_field(&endpoint, "token", token)?,
            );
            map.insert(
                String::from("admin_secret"),
                validate_request_field(&endpoint, "admin_secret", admin_secret)?,
            );
        }
        JanusAdminEndpoint::TestStun => {
            map.insert(
                String::from("admin_secret"),
                validate_request_field(&endpoint, "admin_secret", admin_secret)?,
            );
        }
        _ => {}
    };
    debug!(
        "Building Janus Admin API {:?} request body {:?}",
        &endpoint, &map
    );

    Ok(map)
}

pub async fn janus_admin_api_call(
    host: String,
    endpoint: JanusAdminEndpoint,
    token: Option<String>,
    admin_secret: Option<String>,
) -> Result<String> {
    let body = build_request_body(&endpoint, token, admin_secret)?;
    let client = reqwest::Client::new();
    let res = client.post(host).json(&body).send().await?.text().await?;
    Ok(res)
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

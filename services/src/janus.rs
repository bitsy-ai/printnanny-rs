use anyhow::Result;
use clap::ArgEnum;
use log::info;
use printnanny_api_client::models;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JanusEdgeConfig {
    pub admin_base_path: String,
    pub admin_http_port: i32,
    pub admin_secret: String,
    pub admin_http_url: String,
    pub api_base_path: String,
    pub api_http_port: i32,
    pub api_http_url: String,
    pub api_token: String,
    pub ws_port: i32,
    pub ws_url: String,
}

impl Default for JanusEdgeConfig {
    fn default() -> Self {
        let admin_http_port = 7088;
        let admin_base_path = "/admin".into();
        let admin_secret = "".into();
        let api_http_port = 8088;
        let api_base_path = "janus".into();
        let api_token = "".into();
        let ws_port = 8188;

        let hostname = sys_info::hostname().unwrap_or("localhost".to_string());
        let admin_http_url = format!(
            "http://{}.local:{}{}",
            &hostname, &admin_http_port, &admin_base_path
        )
        .into();
        let api_http_url = format!(
            "http://{}.local:{}{}",
            &hostname, &api_http_port, &api_base_path
        )
        .into();
        let ws_url = format!("http://{}.local:{}/", &hostname, &ws_port).into();

        return Self {
            admin_base_path,
            admin_http_port,
            admin_secret,
            api_http_port,
            api_base_path,
            api_token,
            ws_port,
            admin_http_url,
            api_http_url,
            ws_url,
        };
    }
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

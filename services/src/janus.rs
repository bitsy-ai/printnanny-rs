use anyhow::Result;
use log::info;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

use printnanny_settings::clap;
use printnanny_settings::clap::ValueEnum;

use printnanny_settings::error::PrintNannyCloudDataError;
use printnanny_settings::state::PrintNannyCloudData;

#[derive(Eq, PartialEq, Debug, Clone, Copy, clap::ArgEnum)]
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

pub async fn janus_admin_api_call(endpoint: JanusAdminEndpoint) -> Result<String> {
    let state = PrintNannyCloudData::new()?;
    let err = PrintNannyCloudDataError::SetupIncomplete {
        path: "device.webrtc_edge".to_string(),
    };
    let janus_config = match state.pi {
        Some(pi) => match pi.webrtc_edge {
            Some(webrtc_edge) => Ok(webrtc_edge),
            None => Err(err),
        },
        None => Err(err),
    }?;

    // transaction id
    let transaction: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    let action = endpoint.to_action();

    // build request body
    let mut body = HashMap::new();
    body.insert("transaction", transaction.as_str());
    body.insert("janus", &action);
    info!("Loaded JanusEdgeConfig={:?}", janus_config);
    info!(
        "Building Janus Admin API {:?} request body {:?}",
        &endpoint, &body
    );
    body.insert("admin_secret", &janus_config.admin_secret);

    let token = janus_config.api_token;

    match endpoint {
        JanusAdminEndpoint::AddToken => {
            body.insert("token", &token);
        }
        JanusAdminEndpoint::RemoveToken => {
            body.insert("token", &token);
        }
        _ => {}
    };
    let client = reqwest::Client::new();
    let host = &janus_config.admin_url;
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

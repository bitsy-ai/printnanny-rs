
use std::collections::HashMap;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use anyhow::{ Result };
use clap::arg_enum;
use reqwest;

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum JanusAdminEndpoint {
        GetStatus,
        Info,
        Ping,
        AddToken,
        RemoveToken,
        ListTokens,
        TestStun
    }    
}

impl JanusAdminEndpoint {

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
        format!("{}", action)
    }
}
#[derive(Debug, Clone)]
pub struct JanusAdminService {
    pub host: String,
    pub admin_secret: Option<String>,
    pub token: Option<String>,
}

pub async fn janus_admin_api_call(host: String, endpoint: JanusAdminEndpoint, token: Option<String>, admin_secret: Option<String>) -> Result<String> {
    let action = endpoint.to_action();
    let transaction: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let client = reqwest::Client::new();
    let mut map = HashMap::new();
    map.insert("transaction", &transaction);
    map.insert("janus", &action);
    match endpoint {
        JanusAdminEndpoint::Ping => {
            let body = client.post(host)
                .json(&map)
                .send()
                .await?
                .text()
                .await?;
            Ok(body)
        },
        _ => {Ok("null".to_string())}
    }
}

impl JanusAdminService {

    pub fn new(host: String, admin_secret: Option<String>, token: Option<String>) -> JanusAdminService{
        Self{host, admin_secret, token}
    }

}
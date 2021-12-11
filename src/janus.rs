
use anyhow::{ Result,  anyhow };
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

    pub fn to_url(&self, host: &str) -> String {
        let endpoint = match self {
            JanusAdminEndpoint::GetStatus => "/admin/get_status",
            JanusAdminEndpoint::Info => "/admin/info",
            JanusAdminEndpoint::Ping => "/admin/ping",
            JanusAdminEndpoint::AddToken => "/admin/add_token",
            JanusAdminEndpoint::RemoveToken => "/admin/remove_token",
            JanusAdminEndpoint::ListTokens => "/admin/list_tokens",
            JanusAdminEndpoint::TestStun => "/admin/test_stun",
        };
        format!("{}{}", host, endpoint)
    }
}
#[derive(Debug, Clone)]
pub struct JanusAdminService {
    pub host: String,
    pub admin_secret: Option<String>,
    pub token: Option<String>,
}

pub async fn janus_admin_api_call(host: String, endpoint: JanusAdminEndpoint, token: Option<String>, admin_secret: Option<String>) -> Result<()> {
    let url = endpoint.to_url(&host);
    let res = match endpoint {
        JanusAdminEndpoint::Ping => {
            let body = reqwest::get(url)
                .await?
                .text()
                .await?;
            println!("{}", body)
        },
        _ => {}
    };
    Ok(())
}

impl JanusAdminService {

    pub fn new(host: String, admin_secret: Option<String>, token: Option<String>) -> JanusAdminService{
        Self{host, admin_secret, token}
    }

}
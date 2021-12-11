
use clap::arg_enum;

arg_enum!{
    #[derive(PartialEq, Debug, Clone)]
    pub enum JanusAdminEndpoint {
        Info,
        Ping,
        AddToken,
        RemoveToken,
        ListTokens,
        TestStun
    }    
}
#[derive(Debug, Clone)]
pub struct JanusAdminService {
    pub host: String,
    pub admin_secret: Option<String>,
    pub token: Option<String>
}

impl JanusAdminService {

}
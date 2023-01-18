use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_settings::SettingsFormat;
use std::io::{self, Write};

use printnanny_edge_db::cloud::Pi;

pub struct CloudDataCommand;

impl CloudDataCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        match sub_m.subcommand() {
            Some(("sync", _args)) => {
                let mut service = ApiService::new()?;
                service.sync().await?;
            }
            Some(("show", args)) => {
                let pi = Pi::get()?;
                let v = serde_json::to_vec_pretty(&pi)?;
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|sync|show subcommand"),
        };
        Ok(())
    }
}

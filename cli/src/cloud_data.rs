use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::settings::ConfigFormat;
use printnanny_services::state::PrintNannyCloudData;
use std::io::{self, Write};

pub struct CloudDataCommand;

impl CloudDataCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let config: PrintNannyCloudData = PrintNannyCloudData::new()?;
        match sub_m.subcommand() {
            Some(("sync", _args)) => {
                let mut service = ApiService::new()?;
                service.sync().await?;
            }
            Some(("show", args)) => {
                let f: ConfigFormat = args.value_of_t("format").unwrap();
                let v = match f {
                    ConfigFormat::Json => serde_json::to_vec_pretty(&config)?,
                    ConfigFormat::Toml => toml::ser::to_vec(&config)?,
                };
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|sync|show subcommand"),
        };
        Ok(())
    }
}

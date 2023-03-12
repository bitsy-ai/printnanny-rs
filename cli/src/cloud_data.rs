use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::video_recording_sync::sync_all_video_recordings;
use printnanny_settings::printnanny::PrintNannySettings;
use std::io::{self, Write};

pub struct CloudDataCommand;

async fn on_splitmuxsink_fragment_closed(args: &clap::ArgMatches) -> Result<()> {
    let address = args.value_of("http-address").unwrap();
    let port: i32 = args.value_of_t("http-port").unwrap();
    let factory = PrintNannyPipelineFactory::new(address.into(), port);
    factory
        .on_splitmuxsink_fragment_closed(H264_RECORDING_PIPELINE)
        .await?;
    Ok(())
}

impl CloudDataCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let settings = PrintNannySettings::new().await?;
        match sub_m.subcommand() {
            Some(("sync-models", _args)) => {
                let service = ApiService::from(&settings);
                service.sync().await?;
                service.refresh_nats_creds().await?;
            }

            Some(("sync-videos", _args)) => {
                sync_all_video_recordings().await?;
            }
            Some(("show", _args)) => {
                let service = ApiService::from(&settings);
                let pi = service.pi_retrieve(None).await?;
                let v = serde_json::to_vec_pretty(&pi)?;
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|sync|show subcommand"),
        };
        Ok(())
    }
}

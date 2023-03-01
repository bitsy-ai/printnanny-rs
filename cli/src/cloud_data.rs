use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::video_recording_sync::sync_all_video_recordings;
use printnanny_settings::printnanny::PrintNannySettings;
use std::io::{self, Write};

pub struct CloudDataCommand;

impl CloudDataCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let settings = PrintNannySettings::new().await?;
        match sub_m.subcommand() {
            Some(("sync-models", _args)) => {
                let service = ApiService::from(&settings);
                service.sync().await?;
                service.refresh_nats_creds().await?;
            }

            Some(("sync-video-recordings", args)) => {
                sync_all_video_recordings().await?;
                // TODO
                // let id = args.value_of("id");
                // match id {
                //     Some(id) => sync_video_recording_by_id(id).await?,
                //     None => sync_all_video_recordings().await?,
                // }
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

use printnanny_services::error::ServiceError;
use printnanny_services::printnanny_api::ApiService;
use printnanny_services::video_recording_sync::{
    sync_all_video_recordings, sync_video_recording_by_id,
};
use printnanny_settings::printnanny::PrintNannySettings;
use std::io::{self, Write};

use printnanny_edge_db::cloud::Pi;

pub struct CloudDataCommand;

impl CloudDataCommand {
    pub async fn handle(sub_m: &clap::ArgMatches) -> Result<(), ServiceError> {
        let settings = PrintNannySettings::new().await?;
        let sqlite_connection = settings.paths.db().display().to_string();

        match sub_m.subcommand() {
            Some(("sync-models", _args)) => {
                let service = ApiService::from(&settings);
                service.sync().await?;
            }
            Some(("sync-video-recordings", args)) => {
                let id = args.value_of("id");
                match id {
                    Some(id) => sync_video_recording_by_id(id).await?,
                    None => sync_all_video_recordings().await?,
                }
            }
            Some(("show", _args)) => {
                let pi = Pi::get(&sqlite_connection)?;
                let v = serde_json::to_vec_pretty(&pi)?;
                io::stdout().write_all(&v)?;
            }
            _ => panic!("Expected get|sync|show subcommand"),
        };
        Ok(())
    }
}

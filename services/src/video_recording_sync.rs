use chrono::{DateTime, Utc};
use log::{error, info};
use tokio::task::JoinSet;

use crate::error::VideoRecordingSyncError;
use crate::printnanny_api::ApiService;

use printnanny_edge_db::video_recording;
use printnanny_settings::printnanny::{PrintNannyApiConfig, PrintNannySettings};

pub async fn upload_video_recording_part(
    row: video_recording::VideoRecordingPart,
) -> Result<video_recording::VideoRecordingPart, VideoRecordingSyncError> {
    // create/update cloud model
    let settings = PrintNannySettings::new().await?;
    let sqlite_connection = settings.paths.db().display().to_string();

    let api = ApiService::new(settings.cloud, sqlite_connection.clone());
    let result = api.video_recording_part_create(&row).await?;

    let row = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        &sqlite_connection,
        &row.id,
    )?;

    let sync_start_value = <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<
        DateTime<Utc>,
    >>::into(DateTime::parse_from_rfc3339(&result.sync_start).unwrap());
    let sync_end_value = <chrono::DateTime<chrono::FixedOffset> as std::convert::Into<
        DateTime<Utc>,
    >>::into(DateTime::parse_from_rfc3339(&result.sync_end).unwrap());

    let duration = sync_start_value.signed_duration_since(sync_end_value);
    info!(
        "Finished uploading VideoRecordingPart id={} in ms={}",
        &row.id,
        duration.num_milliseconds(),
    );

    tokio::fs::remove_file(&row.file_name).await?;
    info!(
        "Deleted file VideoRecordingPart id={} file={}",
        &row.id, &row.file_name
    );
    let row = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        &sqlite_connection,
        &row.id,
    )?;
    Ok(row)
}

pub async fn sync_all_video_recordings() -> Result<(), VideoRecordingSyncError> {
    let settings = PrintNannySettings::new().await?;
    let sqlite_connection = settings.paths.db().display().to_string();
    // select all recording parts that have not been uploaded
    let parts = video_recording::VideoRecordingPart::get_ready_for_cloud_sync(&sqlite_connection)?;

    let count = parts.len();
    info!("{} video recording parts ready for cloud sync", count);

    let mut set = JoinSet::new();
    for part in parts {
        set.spawn(upload_video_recording_part(part));
    }

    while let Some(Ok(res)) = set.join_next().await {
        match res {
            Ok(part) => {
                info!("Finished syncing video recording part.id={}", part.id);
            }
            Err(e) => {
                error!("Error syncing video recording part error={}", e);
            }
        }
    }
    info!("Finished syncing {} video recording parts", count);
    Ok(())
}

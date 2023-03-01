use printnanny_settings::printnanny::PrintNannySettings;
use reqwest::Body;
use tokio::fs::File;

use chrono::Utc;
use tokio::task::JoinSet;
use tokio_util::codec::{BytesCodec, FramedRead};

use log::{error, info};

use crate::error::VideoRecordingSyncError;
use crate::printnanny_api::ApiService;

use printnanny_api_client::models;
use printnanny_edge_db::video_recording;
use printnanny_settings::printnanny::PrintNannyApiConfig;

async fn upload_video_recording_part(
    part: video_recording::VideoRecordingPart,
    api_config: PrintNannyApiConfig,
    sqlite_connection: String,
) -> Result<video_recording::VideoRecordingPart, VideoRecordingSyncError> {
    // upload part to PrintNanny OS
    let api = ApiService::new(api_config, sqlite_connection);

    let cloud_part = api
        .video_recording_parts_update_or_create(part.clone().into())
        .await?;
    info!(
        "Uploading part id={} to url={}",
        &cloud_part.id, &cloud_part.mp4_upload_url
    );

    let file = File::open(&part.file_name).await?;
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    let client = reqwest::Client::new();
    let sync_start = Some(Utc::now().to_rfc3339());
    client
        .put(&cloud_part.mp4_upload_url)
        .header("content-type", "application/octet-stream")
        .body(body)
        .send()
        .await?;
    info!("Finished uploading part={}", &cloud_part.id);

    let sync_end = Some(Utc::now().to_rfc3339());
    let req = models::PatchedVideoRecordingPartRequest {
        sync_start,
        sync_end,
        id: None,
        part: None,
        size: None,
        video_recording: None,
    };

    api.video_recording_parts_partial_update(&cloud_part.id, req)
        .await?;
    Ok(part)
    // get or create VideoRecordingPart via cloud API
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
        set.spawn(upload_video_recording_part(
            part,
            settings.cloud.clone(),
            sqlite_connection.clone(),
        ));
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

// pub async fn sync_video_recording_by_id(id: &str) -> Result<(), VideoRecordingSyncError> {
//     let settings = PrintNannySettings::new().await?;
//     let sqlite_connection = settings.paths.db().display().to_string();
//     let video_recording = video_recording::VideoRecording::get_by_id(&sqlite_connection, id)?;
//     let filename = video_recording.mp4_file_name.clone();
//     info!(
//         "Starting cloud sync for VideoRecording: {:?}",
//         &video_recording
//     );
//     generate_upload_url(video_recording).await?;

//     let video_recording = video_recording::VideoRecording::get_by_id(&sqlite_connection, id)?;
//     upload_video_recording(video_recording).await?;

//     info!("Removing local file: {}", &filename);
//     fs::remove_file(&filename)?;
//     let row = UpdateVideoRecording {
//         deleted: Some(&true),
//         cloud_sync_percent: None,
//         cloud_sync_end: None,
//         cloud_sync_status: None,
//         gcode_file_name: None,
//         recording_status: None,
//         recording_start: None,
//         recording_end: None,
//         mp4_upload_url: None,
//         mp4_download_url: None,
//         cloud_sync_start: None,
//     };
//     video_recording::VideoRecording::update(&sqlite_connection, id, row)?;

//     Ok(())
// }

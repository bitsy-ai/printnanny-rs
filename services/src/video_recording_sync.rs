use std::cmp::min;

use futures::stream::StreamExt;
use reqwest::Body;
use tokio::fs::File;

use tokio::task::JoinSet;
use tokio_util::codec::{BytesCodec, FramedRead};

use log::{error, info};

use crate::error::{ServiceError, VideoRecordingSyncError};
use crate::printnanny_api::ApiService;

use printnanny_api_client::models;
use printnanny_edge_db::video_recording;

struct VideoUploadProgress {
    id: String,
    uploaded: u64,
    last_percent: u64,
    interval: u64,
}

fn progress_tick(
    row_id: &str,
    uploaded: u64,
    chunk_size: u64,
    total_size: u64,
    last_emitted_percent: u64,
    interval: u64,
) -> (u64, u64) {
    let uploaded = min(uploaded + chunk_size, total_size);

    let current_percent = total_size / uploaded;
    if current_percent - last_emitted_percent >= interval {
        video_recording::VideoRecording::set_cloud_sync_progress(
            &row_id,
            &(current_percent as i32),
        )
        .expect("Failed to set set_cloud_sync_progress");
        info!(
            "VideoUploadProgress id={} percent={}",
            &row_id, &current_percent
        );
        return (uploaded, current_percent);
    }
    (uploaded, last_emitted_percent)
}

impl VideoUploadProgress {
    pub async fn start(&self) -> Result<(), ServiceError> {
        video_recording::VideoRecording::start_cloud_sync(&self.id)?;
        let row = video_recording::VideoRecording::get_by_id(&self.id)?;
        let api_service = ApiService::new()?;
        api_service.video_recordings_partial_update(&row).await?;
        Ok(())
    }

    // pub fn tick(&mut self, chunk: &[u8]) -> Result<(), VideoRecordingSyncError> {
    //     let uploaded = min(self.uploaded + (chunk.len() as u64), self.total_size);
    //     self.uploaded = uploaded;

    //     let current_percent = self.total_size / uploaded;
    //     if self.last_percent - current_percent >= self.interval {
    //         video_recording::VideoRecording::set_cloud_sync_progress(
    //             &self.id,
    //             &(current_percent as i32),
    //         )?;
    //         info!(
    //             "VideoUploadProgress id={} percent={}",
    //             &self.id, &current_percent
    //         );
    //         self.last_percent = current_percent;
    //     }
    //     Ok(())
    // }

    pub async fn finish(&self) -> Result<(), ServiceError> {
        video_recording::VideoRecording::finish_cloud_sync(&self.id)?;
        let row = video_recording::VideoRecording::get_by_id(&self.id)?;
        let api_service = ApiService::new()?;
        api_service.video_recordings_partial_update(&row).await?;
        Ok(())
    }
}

pub async fn upload_video_recording(
    video_recording: printnanny_edge_db::video_recording::VideoRecording,
) -> Result<video_recording::VideoRecording, VideoRecordingSyncError> {
    let upload_url = match video_recording.mp4_upload_url {
        Some(upload_url) => Ok(upload_url),
        None => Err(VideoRecordingSyncError::UploadUrlNotSet {
            id: video_recording.id.clone(),
            file_name: video_recording.mp4_file_name.clone(),
        }),
    }?;

    let file = File::open(&video_recording.mp4_file_name).await?;
    let total_size = file.metadata().await?.len();

    let mut byte_stream = FramedRead::new(file, BytesCodec::new());
    let mut progress = VideoUploadProgress {
        id: video_recording.id.clone(),
        uploaded: 0,
        last_percent: 0,
        interval: 2, // log progress every 2%
    };

    let row_id = video_recording.id.clone();

    let async_stream = async_stream::stream! {
        match progress.start().await{
            Ok(()) => {},
            Err(e) => error!("Error in VideoUploadProgress.start error={}", e)
        };
        while let Some(chunk) = byte_stream.next().await {
            if let Ok(chunk) = &chunk {
                let chunk_size = chunk.len() as u64;
                let uploaded = progress.uploaded;
                let last_percent = progress.last_percent;
                let interval = progress.interval;
                let row_id = row_id.clone();
                match tokio::task::spawn_blocking(move ||{
                    progress_tick(&row_id, uploaded, chunk_size, total_size, last_percent, interval)
                }).await {
                    Ok((uploaded, last_percent)) => {
                        progress.uploaded = uploaded;
                        progress.last_percent = last_percent;

                    },
                    Err(e) => {
                        error!("Error in VideoUploadProgress.tick error={}", e)
                    }
                }
            }
            yield chunk;
        };
        match progress.finish().await {
            Ok(()) => {},
            Err(e) => error!("Error in VideoUploadProgress.finish error={}", e)
        };
    };
    let body = Body::wrap_stream(async_stream);

    let client = reqwest::Client::new();
    let res = client
        .put(upload_url)
        .header("content-type", "application/octet-stream")
        .body(body)
        .send()
        .await?;
    info!("upload_video_recording response: {:#?}", res);
    let row = video_recording::VideoRecording::get_by_id(&video_recording.id)?;
    Ok(row)
}

async fn generate_upload_url(
    recording: video_recording::VideoRecording,
) -> Result<models::VideoRecording, ServiceError> {
    let api_service = ApiService::new()?;
    let recording = api_service
        .video_recording_update_or_create(&recording)
        .await?;
    Ok(recording)
}

async fn sync_upload_urls(video_recordings: Vec<video_recording::VideoRecording>) -> () {
    // for each video recording, generate a new signed upload url
    let mut set = JoinSet::new();
    for recording in video_recordings {
        set.spawn(generate_upload_url(recording));
    }
    while let Some(Ok(res)) = set.join_next().await {
        match res {
            Ok(recording) => {
                info!(
                    "Fetched upload url for VideoRecording id={:#?} mp4_upload_url={}",
                    &recording.id, recording.mp4_upload_url
                );
            }
            Err(e) => {
                error!("Failed to get upload url for VideoRecording error={}", e);
            }
        }
    }
}

pub async fn handle_sync_video_recordings() -> Result<(), ServiceError> {
    // select all recordings that are finished, but not uploaded
    let video_recordings = video_recording::VideoRecording::get_ready_for_cloud_sync()?;
    info!(
        "Starting cloud sync for VideoRecordings: {:?}",
        &video_recordings
    );
    sync_upload_urls(video_recordings).await;

    // select all recordings that are finished, but not uploaded - which now have an upload url field
    let video_recordings = video_recording::VideoRecording::get_ready_for_cloud_sync()?;

    let mut set = JoinSet::new();
    for recording in video_recordings {
        set.spawn(upload_video_recording(recording));
    }
    while let Some(Ok(res)) = set.join_next().await {
        match res {
            Ok(recording) => {
                let duration = match recording.cloud_sync_start {
                    Some(start) => match recording.cloud_sync_end {
                        Some(end) => Some((end - start).num_seconds()),
                        None => None,
                    },
                    None => None,
                };
                info!(
                    "Finished uploading VideoRecording id={} duration={:#?} seconds",
                    recording.id, duration
                );
            }
            Err(e) => {
                error!("Failed to get upload url for VideoRecording error={}", e);
            }
        }
    }
    Ok(())
}

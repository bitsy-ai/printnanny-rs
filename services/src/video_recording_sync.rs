use std::cmp::min;

use reqwest::Body;
use tokio::fs::File;
use futures::stream::StreamExt;


use tokio_util::codec::{BytesCodec, FramedRead};
use tokio::task::JoinSet;

use log::{warn, info};

use crate::printnanny_api::ApiService;
use crate::error::{ServiceError, VideoRecordingSyncError};

use printnanny_edge_db::diesel;
use printnanny_edge_db::video_recording;

struct VideoUploadProgress {
    id: String,
    total_size: u64,
    uploaded: u64,
    last_percent: u64,
    interval: u64
}

impl VideoUploadProgress {

    pub fn start(&self) -> Result<(), VideoRecordingSyncError> {
        video_recording::VideoRecording::start_cloud_sync(&self.id)?;
        Ok(())
    }


    pub fn tick(&mut self, chunk) -> Result<(), VideoRecordingSyncError> {
        let uploaded = min(self.uploaded + (chunk.len() as u64), self.total_size);
        self.uploaded = uploaded;
        
        let current_percent = self.total_size / uploaded;
        if self.last_percent - current_percent >= self.interval {
            video_recording::VideoRecording::set_cloud_sync_progress(&self.id, &(current_percent as i32))?;
            info!("VideoUploadProgress id={} percent={}", &self.id, &current_percent);
            self.last_percent = current_percent;
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<(), diesel::result::Error> {
        video_recording::VideoRecording::finish_cloud_sync(&self.id)
    }
}

pub async fn upload_video_recording(video_recording: printnanny_edge_db::video_recording::VideoRecording) -> Result<(), VideoRecordingSyncError> {
    
    let upload_url = match video_recording.mp4_upload_url {
        Some(upload_url) => Ok(upload_url),
        None => Err(VideoRecordingSyncError::UploadUrlNotSet{
            id: video_recording.id,
            file_name: video_recording.mp4_file_name
        })
    }?;

    let file = File::open(video_recording.mp4_file_name).await?;
    let total_size = file.metadata().await?.len();

    let byte_stream = FramedRead::new(file, BytesCodec::new());
    let mut progress = VideoUploadProgress {
        id: video_recording.id,
        total_size,
        uploaded: 0,
        last_percent: 0,
        interval: 2 // log progress every 2%
    };
    let mut uploaded = 0;

    let async_stream = async_stream::stream! {
        while let Some(chunk) = byte_stream.next().await {
            if let Ok(chunk) = &chunk {
                tokio::task::spawn_blocking(||{
                    progress.tick(chunk)
                }).await?;
            }
            yield chunk;
        }
        progress.finish();
    };
    let body = Body::wrap_stream(async_stream);

    let client = reqwest::Client::new();
    let res = client
        .put(upload_url)
        .header("content-type", "application/octet-stream")
        .body(body)
        .send()
        .await?;
    Ok(())
}



pub async fn generate_upload_url(recording: video_recording::VideoRecording) -> Result<String, ServiceError>{
    let api_service = ApiService::new()?;
    let recording = api_service.video_recording_update_or_create(recording).await?;
    Ok(recording.mp4_upload_url)
}

pub async fn handle_sync_video_recordings() {
    // select all recordings that are finished, but not uploaded
    let mut video_recordings = video_recordings::get_ready_for_cloud_sync();

    // for each video recording, generate a new signed upload url
    let mut set = JoinSet::new();
    for recording in video_recordings {
        set.spawn(generate_upload_url(recording));
    }

    // // submit VideoRecording to PrintNanny Cloud API to get an upload url with 48H expiration
    // for recording in video_recordings {
    //     set.spawn(upload_video_recording(recording));
    // }
}

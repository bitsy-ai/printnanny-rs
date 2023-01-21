use std::cmp::min;

use reqwest::Body;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

use tokio::fs::File;
use tokio::io::AsyncRead;
use tokio_util::codec::{BytesCodec, FramedRead};

use log::{warn, info};

use printnanny_edge_db::video_recording;

struct VideoUploadProgress {
    id: String,
    total_size: u64,
    uploaded: u64,
    last_percent: u64,
    interval: u64
}

impl VideoUploadProgress {

    pub fn start(&self) -> Result<()> {
        video_recording::VideoRecording::start_cloud_sync(&self.id)?;
        Ok(())
    }


    pub fn tick(&mut self, chunk) -> Result<()> {
        let uploaded = min(self.uploaded + (chunk.len() as u64), self.total_size);
        self.uploaded = uploaded;
        
        let current_percent = self.total_size / uploaded;
        if self.last_percent - current_percent >= self.interval {
            video_recording::VideoRecording::set_cloud_sync_progress(&self.id, &current_percent)?;
            info!("VideoUploadProgress id={} percent={}", &self.id, &current_percent);
            self.last_percent = current_percent;
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        video_recording::VideoRecording::finish_cloud_sync(&self.id)
    }
}

pub async fn upload_video_recording(src_id: String, src_file: &str, upload_url: &str) -> Result<()> {
    // tokio::fs::File will use a streaming reader
    let file = File::open(src_file).await?;
    let total_size = file.metadata().unwrap().len();

    let byte_stream = FramedRead::new(file, BytesCodec::new());
    let mut progress = VideoUploadProgress {
        id: src_id,
        total_size,
        uploaded: 0,
        last_percent: 0,
        interval: 2 // log progress every 2%
    };
    let mut uploaded = 0;

    let async_stream = async_stream::stream! {
        while let Some(chunk) = byte_stream.next().await {
            if let Ok(chunk) = &chunk {
                progress.tick(chunk)
            }
            yield chunk;
        }
        pb.finish_with_message(format!("Uploaded {} to {}", url, path));
    };
    let body = Body::wrap_stream(stream);

    let client = reqwest::Client::new();
    let res = client
        .put(upload_url)
        .header("content-type", "application/octet-stream")
        .body(vec)
        .send()
        .await?;
}

pub async fn handle_sync_video_recordings() {
    // select all recordings that are finished, but not uploaded
    let video_recordings = video_recordings::get_ready_for_cloud_sync();

    // submit VideoRecording to PrintNanny Cloud API to get an upload url with 48H expiration
    for recording in video_recordings {
        tokio::spawn
    }
}

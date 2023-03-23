#[macro_use]
extern crate clap;

use anyhow::Result;
use clap::{Arg, Command};
use printnanny_services::printnanny_api::ApiService;
use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use env_logger::Builder;
use git_version::git_version;
use log::{error, info, LevelFilter};
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

use printnanny_api_client::models;
use printnanny_gst_pipelines::factory::{PrintNannyPipelineFactory, H264_RECORDING_PIPELINE};
use printnanny_gst_pipelines::gst_client;

use gst_client::gstd_types::{
    GstSplitMuxSinkFragmentClosed, GstSplitMuxSinkFragmentMessage,
    GST_SPLIT_MUX_SINK_FRAGMENT_MESSAGE_CLOSED,
};

use printnanny_nats_client::client::wait_for_nats_client;
use printnanny_nats_client::event::NatsEventHandler;

use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::sys_info;

use printnanny_nats_apps::event::NatsEvent;

const SUBJECT_PATTERN: &str = "pi.{pi_id}.event.camera.recording.part";
const DEFAULT_NATS_URI: &str = "nats://localhost:4223";
const DEFAULT_NATS_WAIT: u64 = 2000; // sleep 2 seconds between connection attempts
const GST_BUS_TIMEOUT: u64 = 600000000000_u64; // 600 seconds (in nanoseconds)
const GIT_VERSION: &str = git_version!();

// parse recording id from path like: /home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4
fn parse_video_recording_id(filename: &str) -> String {
    let path = PathBuf::from(filename);
    let mut components = path.components();
    components
        .nth_back(1)
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap()
        .into()
}

fn parse_video_recording_index(filename: &str) -> i32 {
    let path = PathBuf::from(filename);
    let mut components = path.components();
    let last: String = components
        .nth_back(0)
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap()
        .into();
    last.split(".").nth(0).unwrap().parse().unwrap()
}

// Insert local VideoRecordingPart row
fn handle_filesink_msg_opened(
    filesink_msg: GstSplitMuxSinkFragmentMessage,
    sqlite_connection: &str,
) -> Result<printnanny_edge_db::video_recording::VideoRecordingPart> {
    // parse recording id from filesink_msg
    let video_recording_id = parse_video_recording_id(&filesink_msg.location);

    let size = fs::metadata(&filesink_msg.location)?.len() as i64;
    let index = parse_video_recording_index(&filesink_msg.location);

    // insert new VideoRecordingPart
    let row_id = format!("{video_recording_id}-{index}");
    let row = printnanny_edge_db::video_recording::NewVideoRecordingPart {
        id: &row_id,
        buffer_index: &index,
        buffer_runningtime: &(filesink_msg.running_time as i64),
        deleted: &false,
        file_name: &filesink_msg.location,
        video_recording_id: &video_recording_id,
        size: &size,
    };
    match printnanny_edge_db::video_recording::VideoRecordingPart::insert(sqlite_connection, row) {
        Ok(()) => info!(
            "Inserted VideoRecordingPart video_recording_id={} id={} file_name={}",
            &video_recording_id, &row_id, &filesink_msg.location
        ),
        Err(e) => error!("Failed to insert VideoRecordingPart row, error={}", e),
    }
    let result = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        sqlite_connection,
        &row_id,
    )?;
    Ok(result)
}

async fn handle_file_upload(url: &str, filename: &str) -> Result<()> {
    let mut file = File::open(filename).await?;
    let mut vec = Vec::new();
    file.read_to_end(&mut vec);
    let client = reqwest::Client::new();
    let res = client
        .put(url)
        .header("content-type", "application/octet-stream")
        .body(vec)
        .send()
        .await?;
    Ok(())
}

// upload VideoRecordingPart and publish NATS message
async fn handle_filesink_msg_closed(
    filesink_msg: GstSplitMuxSinkFragmentMessage,
    sqlite_connection: &str,
) -> Result<printnanny_edge_db::video_recording::VideoRecordingPart> {
    // parse recording id from filesink_msg
    let video_recording_id = parse_video_recording_id(&filesink_msg.location);

    let size = fs::metadata(&filesink_msg.location)?.len() as i64;
    let index = parse_video_recording_index(&filesink_msg.location);
    let row_id = format!("{video_recording_id}-{index}");

    let start = Utc::now();

    let row = printnanny_edge_db::video_recording::UpdateVideoRecordingPart {
        sync_start: Some(&start),
        deleted: None,
        sync_end: None,
    };
    // update local model
    printnanny_edge_db::video_recording::VideoRecordingPart::update(
        sqlite_connection: &str,
        &row_id,
        row,
    )?;
    let row = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        sqlite_connection,
        &row_id,
    )?;

    // create/update cloud model
    let settings = PrintNannySettings::new().await?;
    let api = ApiService::new(settings.cloud, sqlite_connection.to_string());
    let remote = api
        .video_recording_parts_update_or_create(row.into())
        .await?;

    handle_file_upload(&row.file_name, &url).await?;
    let end = Utc::now();
    let duration = end.signed_duration_since(start);
    info!(
        "Finished uploading VideoRecordingPart id={} in ms={}",
        &row_id,
        duration.num_milliseconds(),
    );

    tokio::fs::remove_file(&filesink_msg.location).await?;
    info!(
        "Deleted file VideoRecordingPart id={} file={}",
        &row_id, &filesink_msg.location
    );

    let request = models::PatchedVideoRecordingPartRequest {
        id: None,
        sync_end: Some(end.to_rfc3339()),
        size: None,
        buffer_index: None,
        buffer_runningtime: None,
        file_name: None,
        sync_start: None,
        video_recording: None,
    };
    let result = api
        .video_recording_parts_partial_update(&row_id, request)
        .await?;
    let row = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        sqlite_connection,
        &row_id,
    )?;

    Ok(row)
}

// subscribe to splitmuxsink-fragment-closed message
async fn run_splitmuxsink_fragment_publisher(
    factory: PrintNannyPipelineFactory,
    pipeline_name: &str,
    hostname: &str,
) -> Result<()> {
    let settings = PrintNannySettings::new().await?;
    let sqlite_connection = settings.paths.db().display().to_string();
    let nats_client =
        wait_for_nats_client(DEFAULT_NATS_URI, &None, false, DEFAULT_NATS_WAIT).await?;
    let client = factory.gst_client();
    let pipeline = client.pipeline(pipeline_name);
    let bus = pipeline.bus();
    let subject: String = NatsEvent::replace_subject_pattern(SUBJECT_PATTERN, hostname, "{pi_id}");

    // filter bus messages
    info!("Setting gstd filter pipeline={pipeline_name} filter=element");
    bus.set_filter("element").await?;

    // set timeout
    info!("Setting timeout on pipeline={pipeline_name} timeout={GST_BUS_TIMEOUT}");
    bus.set_timeout(GST_BUS_TIMEOUT).await?;
    // read bus messagesz
    info!(
        "Waiting for msg={} on pipeline={}",
        GST_SPLIT_MUX_SINK_FRAGMENT_MESSAGE_CLOSED, pipeline_name
    );

    loop {
        let msg = bus.read().await;
        info!("Received msg={:?}", msg);
        match msg {
            Ok(msg) => match msg.response {
                gst_client::gstd_types::ResponseT::GstSplitMuxSinkFragmentOpened(msg) => {
                    info!(
                        "Handling msg on gstreamer pipeline bus name={} msg={:?}",
                        pipeline_name, msg
                    );
                    // insert filesink msg row
                    let result = handle_filesink_msg_opened(msg.message, &sqlite_connection);
                    match result {
                        Ok(result) => {
                            info!("Inserted VideoRecordingPart id={}", result.id);
                        }
                        Err(e) => {
                            error!("Failed to insert VideoRecordingPart row error={}", e)
                        }
                    }
                }
                gst_client::gstd_types::ResponseT::GstSplitMuxSinkFragmentClosed(msg) => {
                    info!(
                        "Handling msg on gstreamer pipeline bus name={} msg={:?}",
                        pipeline_name, msg
                    );
                    // insert filesink msg row
                    let result = handle_filesink_msg_closed(msg.message, &sqlite_connection).await;
                    match result {
                        Ok(result) => {
                            // publish NATS message
                            let payload = serde_json::to_vec(&result)?;
                            nats_client.publish(subject.clone(), payload.into()).await?;
                            info!("Published subject={} id={}", &subject, &result.id)
                        }
                        Err(e) => {
                            error!("Failed to upload VideoRecordingPart row error={}", e)
                        }
                    }
                }
                _ => error!("Failed to process response={:#?}", msg.response),
            },
            Err(e) => {
                error!("Error reading gstreamer pipeline bus name={} filter=splitmuxsink-fragment-closed error={}", pipeline_name, e);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::new();
    let app_name = "printnanny-nats-gstmultifile";
    let hostname = sys_info::hostname()?;

    let app = Command::new(app_name)
        .author(crate_authors!())
        .about(crate_description!())
        .version(GIT_VERSION)
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity. Info: -v Debug: -vv Trace: -vvv"),
        )
        .arg(
            Arg::new("http-address")
                .takes_value(true)
                .long("http-address")
                .default_value("127.0.0.1")
                .help("Attach to the server through a given address"),
        )
        .arg(
            Arg::new("http-port")
                .takes_value(true)
                .long("http-port")
                .default_value("5002")
                .help("Attach to the server through a given port"),
        )
        .arg(
            Arg::new("hostname")
                .long("hostname")
                .default_value(&hostname)
                .takes_value(true),
        )
        .arg(
            Arg::new("pipeline")
                .takes_value(true)
                .long("pipeline")
                .default_value(H264_RECORDING_PIPELINE)
                .help("Name of pipeline"),
        );
    let args = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = args.occurrences_of("v");
    match verbosity {
        0 => {
            builder.filter_level(LevelFilter::Warn).init();
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
        }
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    let factory = PrintNannyPipelineFactory::from(&args);
    let pipeline = args.value_of("pipeline").unwrap();
    let hostname = args.value_of("hostname").unwrap();

    factory.wait_for_pipeline(pipeline).await?;
    run_splitmuxsink_fragment_publisher(factory, pipeline, hostname).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_video_recording_id() {
        let filename = "/home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4";
        let expected = "66b3a3a0-30b5-41f2-9907-a335de57c921";
        let result = parse_video_recording_id(filename);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_video_recording_index() {
        let filename = "/home/printnanny/.local/share/printnanny/video/66b3a3a0-30b5-41f2-9907-a335de57c921/00025.mp4";
        let expected = 25;
        let result = parse_video_recording_index(filename);
        assert_eq!(result, expected);
    }
}

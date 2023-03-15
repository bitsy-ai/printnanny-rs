#[macro_use]
extern crate clap;

use anyhow::{anyhow, Result};
use clap::{Arg, Command};
use std::fs;

use env_logger::Builder;
use git_version::git_version;
use log::{error, info, LevelFilter};

use printnanny_gst_pipelines::factory::{PrintNannyPipelineFactory, H264_RECORDING_PIPELINE};
use printnanny_gst_pipelines::gst_client;
use printnanny_gst_pipelines::message::{
    GstMultiFileSinkMessage, GstSplitMuxSinkFragmentMessage,
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
const GST_BUS_TIMEOUT: i32 = 6e+11 as i32; // 600 seconds (in nanoseconds)
const GIT_VERSION: &str = git_version!();

// Subscribe to GstMultiFileSink messages on gstreamer bus, re-publish NATS message
fn handle_filesink_msg(
    filesink_msg: GstMultiFileSinkMessage,
    sqlite_connection: &str,
) -> Result<printnanny_edge_db::video_recording::VideoRecordingPart> {
    // try to get current recording
    let recording =
        printnanny_edge_db::video_recording::VideoRecording::get_current(sqlite_connection)?;
    if recording.is_none() {
        return Err(anyhow!(
            "Refusing to process GstMultiFileSink msg, could not find active recording"
        ));
    }
    let video_recording_id = recording.unwrap().id;

    let size = fs::metadata(&filesink_msg.filename)?.len() as i64;

    // insert new VideoRecordingPart
    let row_id = format!("{video_recording_id}-{}", filesink_msg.index);
    let row = printnanny_edge_db::video_recording::NewVideoRecordingPart {
        id: &row_id,
        buffer_index: &(filesink_msg.index as i32),
        buffer_ts: &(filesink_msg.timestamp as i64),
        buffer_streamtime: &(filesink_msg.streamtime as i64),
        buffer_runningtime: &(filesink_msg.runningtime as i64),
        buffer_duration: &(filesink_msg.duration as i64),
        buffer_offset: &(filesink_msg.offset as i64),
        buffer_offset_end: &(filesink_msg.offset_end as i64),
        deleted: &false,
        file_name: &filesink_msg.filename,
        video_recording_id: &video_recording_id,
        size: &size,
    };
    match printnanny_edge_db::video_recording::VideoRecordingPart::insert(sqlite_connection, row) {
        Ok(()) => info!(
            "Inserted VideoRecordingPart video_recording_id={} id={} file_name={}",
            &video_recording_id, &row_id, &filesink_msg.filename
        ),
        Err(e) => error!("Failed to insert VideoRecordingPart row, error={}", e),
    }
    let result = printnanny_edge_db::video_recording::VideoRecordingPart::get_by_id(
        sqlite_connection,
        &row_id,
    )?;
    Ok(result)
}

// subscribe to splitmuxsink-fragment-closed message
async fn run_splitmuxsink_fragment_publisher(
    factory: PrintNannyPipelineFactory,
    pipeline_name: &str,
    hostname: &str,
) -> Result<()> {
    let settings = PrintNannySettings::new().await?;
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
                gst_client::gstd_types::ResponseT::Bus(Some(msg)) => {
                    info!(
                        "Handling msg on gstreamer pipeline bus name={} msg={:?}",
                        pipeline_name, msg
                    );

                    // attempt to deserialize msg
                    let filesink_msg =
                        serde_json::from_str::<GstSplitMuxSinkFragmentMessage>(&msg.message);
                    match filesink_msg {
                        Ok(filesink_msg) => {
                            // insert filesink msg row
                            info!("Deserialized msg: {:?}", filesink_msg);
                            // let result = handle_filesink_msg(filesink_msg, &sqlite_connection);
                            // match result {
                            //     Ok(result) => {
                            //         // publish NATS message
                            //         let payload = serde_json::to_vec(&result)?;
                            //         nats_client.publish(subject.clone(), payload.into()).await?;
                            //         info!("Published subject={} id={}", &subject, &result.id)
                            //     }
                            //     Err(e) => {
                            //         error!("Failed to insert VideoRecordingPart row error={}", e)
                            //     }
                            // }
                        }
                        Err(e) => {
                            error!(
                                "Failed to deserialize GstSplitMuxSinkFragmentMessage from msg={} error={}",
                                &msg.message,
                                e
                            );
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

async fn run_multifilesink_fragment_publisher(
    factory: PrintNannyPipelineFactory,
    pipeline_name: &str,
    hostname: &str,
) -> Result<()> {
    let settings = PrintNannySettings::new().await?;

    let nats_client =
        wait_for_nats_client(DEFAULT_NATS_URI, &None, false, DEFAULT_NATS_WAIT).await?;

    let client = gst_client::GstClient::build(&factory.uri).expect("Failed to build GstClient");
    let pipeline = client.pipeline(pipeline_name);
    let bus = pipeline.bus();
    let subject: String = NatsEvent::replace_subject_pattern(SUBJECT_PATTERN, hostname, "{pi_id}");

    // filter bus messages
    bus.set_filter("GstMultiFileSink").await?;

    // set timeout
    bus.set_timeout(GST_BUS_TIMEOUT).await?;

    // read bus messages
    info!(
        "Set filter for messages=GstMultiFileSink on pipeline={}",
        pipeline_name
    );

    let sqlite_connection = settings.paths.db().display().to_string();
    loop {
        let msg = bus.read().await;
        match msg {
            Ok(msg) => {
                match msg.response {
                    gst_client::gstd_types::ResponseT::Bus(Some(msg)) => {
                        info!(
                            "Handling msg on gstreamer pipeline bus name={} msg={:?}",
                            pipeline_name, msg
                        );

                        // attempt to deserialize msg
                        let filesink_msg =
                            serde_json::from_str::<GstMultiFileSinkMessage>(&msg.message);
                        match filesink_msg {
                            Ok(filesink_msg) => {
                                // insert filesink msg row
                                let result = handle_filesink_msg(filesink_msg, &sqlite_connection);
                                match result {
                                    Ok(result) => {
                                        // publish NATS message
                                        let payload = serde_json::to_vec(&result)?;
                                        nats_client
                                            .publish(subject.clone(), payload.into())
                                            .await?;
                                        info!("Published subject={} id={}", &subject, &result.id)
                                    }
                                    Err(e) => error!(
                                        "Failed to insert VideoRecordingPart row error={}",
                                        e
                                    ),
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to deserialize GstMultiFileSinkMessage from msg={} error={}",
                                    &msg.message,
                                    e
                                );
                            }
                        }
                    }
                    _ => error!("Failed to process response={:#?}", msg.response),
                }
            }
            Err(e) => {
                error!("Error reading gstreamer pipeline bus name={} filter=splitmuxsink-fragment-closed error={}", pipeline_name, e);
            }
        }
    }
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
    // run_multifilesink_fragment_publisher(factory, pipeline, hostname).await?;
    run_splitmuxsink_fragment_publisher(factory, pipeline, hostname).await?;

    Ok(())
}

use anyhow::Result;
use printnanny_gst_pipelines::factory::PrintNannyPipelineFactory;
use printnanny_nats::subscriber::wait_for_nats_client;

use printnanny_gst_pipelines::message::GstMultiFileSinkMessage;

const DEFAULT_NATS_URI: &str = "nats://localhost:4223";

// Subscribe to GstMultiFileSink messages on gstreamer bus, re-publish NATS message

// subscribe to splitmuxsink-fragment-closed message
pub async fn run_multifilesink_fragment_publisher(pipeline_name: &str) -> Result<()> {
    let settings = PrintNannySettings::new().await?;

    let nats_client = wait_for_nats_client(DEFAULT_NATS_URI, None, false).await?;

    let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
    let pipeline = client.pipeline(pipeline_name);
    let bus = pipeline.bus();
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
                    gstd_types::ResponseT::Bus(Some(msg)) => {
                        info!(
                            "Handling msg on gstreamer pipeline bus name={} msg={:?}",
                            pipeline_name, msg
                        );

                        // attempt to deserialize msg
                        let filesink_msg =
                            serde_json::from_str::<GstMultiFileSinkMessage>(&msg.message);
                        match filesink_msg {
                            Ok(filesink_msg) => {
                                // try to get current recording
                                let recording =
                                    printnanny_edge_db::video_recording::VideoRecording::get_current(&sqlite_connection)?;
                                if recording.is_none() {
                                    warn!("Refusing to process GstMultiFileSink msg, could not find active recording");
                                    continue;
                                }
                                let video_recording_id = recording.unwrap().id;

                                let size = fs::metadata(&filesink_msg.filename)?.len() as i64;

                                // insert new VideoRecordingPart
                                let row_id = format!("{video_recording_id}-{}", filesink_msg.index);
                                let row =
                                    printnanny_edge_db::video_recording::NewVideoRecordingPart {
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
                                match printnanny_edge_db::video_recording::VideoRecordingPart::insert(&sqlite_connection, row) {
                                        Ok(()) => info!("Inserted VideoRecordingPart video_recording_id={} id={} file_name={}",&video_recording_id, &row_id, &filesink_msg.filename  ),
                                        Err(e) => error!("Failed to insert VideoRecordingPart row, error={}", e)
                                    }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to deserialize GstMultiFileSinkMessage from msg={}",
                                    &msg.message
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

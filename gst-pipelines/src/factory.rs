use std::fs;

use anyhow::Result;
use clap::ArgMatches;
use gst_client::reqwest;
use gst_client::GstClient;
use log::{debug, error, info, warn};
use tokio::time::{sleep, Duration};

use printnanny_settings::cam::VideoStreamSettings;
use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::printnanny_os_models::CameraSettings;

pub const CAMERA_PIPELINE: &str = "camera";
pub const H264_ENCODING_PIPELINE: &str = "h264_encode";
pub const RTP_PIPELINE: &str = "rtp";
pub const INFERENCE_PIPELINE: &str = "tflite_inference";
pub const BB_PIPELINE: &str = "bounding_boxes";
pub const DF_WINDOW_PIPELINE: &str = "df";
pub const SNAPSHOT_PIPELINE: &str = "snapshot";
pub const HLS_PIPELINE: &str = "hls";
pub const H264_RECORDING_PIPELINE: &str = "h264_record";
pub const H264_SPLITMUXSINK: &str = "h264_splitmuxsink";

#[derive(Clone, Debug)]
pub struct PrintNannyPipelineFactory {
    pub address: String,
    pub port: i32,
    pub uri: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GstPipelineState {
    Paused,
    Playing,
    Ready,
    Null,
}

impl From<&str> for GstPipelineState {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_ref() {
            "playing" => GstPipelineState::Playing,
            "paused" => GstPipelineState::Paused,
            "ready" => GstPipelineState::Ready,
            "null" => GstPipelineState::Null,
            _ => GstPipelineState::Null,
        }
    }
}

impl Default for PrintNannyPipelineFactory {
    fn default() -> Self {
        let address = "127.0.0.1".to_string();
        let port = 5002;
        let uri = Self::uri(&address, port);
        Self { address, port, uri }
    }
}

impl From<&ArgMatches> for PrintNannyPipelineFactory {
    fn from(args: &ArgMatches) -> Self {
        let defaults = PrintNannyPipelineFactory::default();
        let address = args.value_of("http-address").unwrap_or(&defaults.address);
        let port: i32 = args.value_of_t("http-port").unwrap_or(defaults.port);
        Self::new(address.to_string(), port)
    }
}

impl PrintNannyPipelineFactory {
    pub fn new(address: String, port: i32) -> Self {
        let uri = Self::uri(&address, port);
        Self { address, port, uri }
    }
    pub fn uri(address: &str, port: i32) -> String {
        format!("http://{}:{}", address, port)
    }

    pub fn gst_client(&self) -> GstClient {
        GstClient::build(&self.uri).expect("Failed to build GstClient")
    }

    pub async fn pipeline_state(&self, pipeline_name: &str) -> GstPipelineState {
        let client = self.gst_client();
        match client.pipeline(pipeline_name).state().await {
            Ok(state_res) => match state_res.response {
                gst_client::gstd_types::ResponseT::Property(prop) => match prop.value {
                    gst_client::gstd_types::PropertyValue::String(state) => {
                        GstPipelineState::from(state.as_ref())
                    }
                    _ => GstPipelineState::Null,
                },
                _ => GstPipelineState::Null,
            },
            Err(e) => {
                // H264_RECORDING_PIPELINE state is polled every N seconds, 404ing when pipeline doesn't exist
                // log these at the debug! level, and all other pipelines at the error! level
                match pipeline_name {
                    H264_RECORDING_PIPELINE => debug!(
                        "Error getting gst pipeline state name={} error={}",
                        pipeline_name, e
                    ),
                    _ => error!(
                        "Error getting gst pipeline state name={} error={}",
                        pipeline_name, e
                    ),
                }
                GstPipelineState::Null
            }
        }
    }

    fn to_interpipesrc_name(pipeline_name: &str) -> String {
        format!("{pipeline_name}_src")
    }

    fn to_interpipesink_name(pipeline_name: &str) -> String {
        format!("{pipeline_name}_sink")
    }

    async fn delete_pipeline(&self, pipeline_name: &str) -> Result<gst_client::Response> {
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(pipeline_name);
        Ok(pipeline.delete().await?)
    }

    async fn make_pipeline(
        &self,
        pipeline_name: &str,
        description: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        info!(
            "Creating {} pipeline with description: {}",
            pipeline_name, &description
        );
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(pipeline_name);
        match pipeline.create(description).await {
            Ok(result) => {
                info!("Created pipeline={}: {:?}", pipeline_name, result);
                Ok(())
            }
            Err(e) => {
                error!("Error creating pipeline name={} error={}", pipeline_name, e);
                match e {
                    gst_client::Error::BadStatus(reqwest::StatusCode::CONFLICT, ref body) => {
                        info!(
                            "Pipeline with name={} already exists, body={:?}",
                            pipeline_name, body
                        );
                        Ok(())
                    }
                    _ => Err(e),
                }
            }
        }?;
        Ok(pipeline)
    }

    // wait for pipeline to be available
    pub async fn wait_for_pipeline(&self, pipeline_name: &str) -> Result<()> {
        let wait = 2000;
        warn!("Waiting for {} to become available", pipeline_name);
        let mut status = self.pipeline_state(pipeline_name).await;
        while status != GstPipelineState::Playing {
            debug!(
                "Pipeline {} unavailable, status={:?} waiting {} ms",
                pipeline_name, status, wait
            );
            sleep(Duration::from_millis(wait)).await;
            status = self.pipeline_state(pipeline_name).await;
        }
        warn!("Pipeline {} is now available", pipeline_name);
        Ok(())
    }

    async fn make_camera_pipeline(
        &self,
        pipeline_name: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesink = Self::to_interpipesink_name(pipeline_name);
        let caps = settings.gst_camera_caps();

        let description = format!(
            "libcamerasrc camera-name={camera_name} \
            ! capsfilter caps={caps} \
            ! v4l2convert \
            ! interpipesink name={interpipesink} sync=true async=false",
            camera_name = settings.camera.device_name,
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_jpeg_snapshot_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);

        let filesink_location = settings.snapshot.path.as_str();

        let max_buffers = 30;
        let caps = settings.gst_camera_caps();
        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false max-buffers={max_buffers} leaky-type=2 caps={caps} \
            ! v4l2jpegenc ! multifilesink location={filesink_location} max-files={max_buffers}",
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_h264_encode_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        let caps: String = settings.gst_camera_caps();
        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true caps={caps} \
            ! v4l2h264enc extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse name={pipeline_name}_h264parse \
            ! capssetter caps=video/x-h264,level=(string)4,profile=(string)high \
            ! interpipesink name={interpipesink} sync=false async=false forward-events=true forward-eos=true",
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_rtp_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

        let port = settings.rtp.video_udp_port;

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true format=3 \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
            ! queue2 \
            ! udpsink port={port}");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_hls_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

        // use time-based segment format for rtp and hls pipelines
        // format              : The format of the segment events and seek
        // flags: readable, writable
        // Enum "GstFormat" Default: 2, "bytes"
        //    (0): undefined        - GST_FORMAT_UNDEFINED
        //    (1): default          - GST_FORMAT_DEFAULT
        //    (2): bytes            - GST_FORMAT_BYTES
        //    (3): time             - GST_FORMAT_TIME
        //    (4): buffers          - GST_FORMAT_BUFFERS
        //    (5): percent          - GST_FORMAT_PERCENT
        let hls_settings = &*settings.hls;
        let hls_segments_location = hls_settings.segments.as_str();
        let hls_playlist_location = hls_settings.playlist.as_str();
        let hls_playlist_root = hls_settings.playlist_root.as_str();
        let framerate_n = settings.camera.framerate_n;
        let target_duration = (60 / framerate_n) + 1; // v4l2-ctl --list-ctrls-menu -d 11 -> h264_i_frame_period default sends a key unit every 60 frames

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true format=3 \
            ! hlssink2 playlist-length=8 max-files=10 target-duration={target_duration} location={hls_segments_location} playlist-location={hls_playlist_location} playlist-root={hls_playlist_root} send-keyframe-requests=false");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_inference_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        let tensor_format = "RGB"; // model expects pixel data to be in RGB format
        let caps: String = settings.gst_camera_caps();

        let detection_settings = &*settings.detection;
        let tensor_width = detection_settings.tensor_width;
        let tensor_height = detection_settings.tensor_height;
        let tflite_model_file = detection_settings.model_file.as_str();

        let max_buffers = 3;
        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false max-buffers={max_buffers} leaky-type=2 caps={caps} \
            ! v4l2convert ! videoscale ! capsfilter caps=video/x-raw,format={tensor_format},width={tensor_width},height={tensor_height} \
            ! tensor_converter \
            ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1 \
            ! capsfilter caps=other/tensors,format=static \
            ! tensor_filter framework=tensorflow2-lite model={tflite_model_file} \
            ! interpipesink name={interpipesink} sync=false async=false",
        );

        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_bounding_box_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

        let port = settings.rtp.overlay_udp_port;
        let detection = &(*settings.detection);

        // let colorimetry = "bt709";

        // use time-based segment format for rtp and hls pipelines
        // format              : The format of the segment events and seek
        // flags: readable, writable
        // Enum "GstFormat" Default: 2, "bytes"
        //    (0): undefined        - GST_FORMAT_UNDEFINED
        //    (1): default          - GST_FORMAT_DEFAULT
        //    (2): bytes            - GST_FORMAT_BYTES
        //    (3): time             - GST_FORMAT_TIME
        //    (4): buffers          - GST_FORMAT_BUFFERS
        //    (5): percent          - GST_FORMAT_PERCENT
        let caps: String = settings.gst_tensor_decoder_caps();
        let camera = &*settings.camera;

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=true \
            ! tensor_decoder name=bb_tensor_decoder mode=bounding_boxes option1=mobilenet-ssd-postprocess option2={tflite_label_file} option3=0:1:2:3,{nms_threshold} option4={video_width}:{video_height} option5={tensor_width}:{tensor_height} \
            ! queue \
            ! v4l2convert \
            ! capsfilter caps={caps} \
            ! v4l2h264enc extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse \
            ! capssetter caps=video/x-h264,level=(string)4,profile=(string)high \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
            ! udpsink port={port}",
            nms_threshold=detection.nms_threshold,
            tflite_label_file=detection.label_file,
            tensor_height=detection.tensor_height,
            tensor_width=detection.tensor_width,
            video_width=camera.width,
            video_height=camera.height,

        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_df_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        settings: &VideoStreamSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let detection = &(*settings.detection);

        let nms_threshold = detection.nms_threshold as f32 / 100_f32;
        let nats_server_uri = detection.nats_server_uri.as_str();

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false \
            ! tensor_decoder name=df_tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
            ! dataframe_agg filter-threshold={nms_threshold} output-type=json \
            ! nats_sink nats-address={nats_server_uri}");
        self.make_pipeline(pipeline_name, &description).await
    }
    async fn make_recording_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filename: &str,
        filesink_name: &str,
        _camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);

        // ensure directory exists
        match fs::create_dir_all(filename) {
            Ok(_) => {
                info!("Created directory={}", filename);
            }
            Err(e) => {
                error!("Error creating directory={} error={}", filename, e);
            }
        };

        let location = format!("{filename}/%d.mp4");
        let max_files = 50;

        let max_bytes = 10000000; // 10MB (bytes)

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=true is-live=true allow-renegotiation=true format=3 stream-sync=passthrough-ts \
            ! queue \
            ! splitmuxsink muxer=mpegtsmux name={filesink_name} max-files={max_files} location={location} max-size-bytes={max_bytes} send-keyframe-requests=false");
        self.make_pipeline(pipeline_name, &description).await
    }

    pub async fn stop_pipeline(&self, pipeline_name: &str) -> Result<()> {
        info!("Attempting to stop Gstreamer pipeline: {}", &pipeline_name);
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(pipeline_name);
        pipeline.stop().await?;
        info!("Success! Stopped Gstreamer pipeline: {}", &pipeline_name);
        Ok(())
    }

    pub async fn start_pipeline(&self, pipeline_name: &str) -> Result<()> {
        info!("Attempting to start Gstreamer pipeline: {}", &pipeline_name);
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(pipeline_name);
        pipeline.pause().await?;
        pipeline.play().await?;
        info!("Success! Started Gstreamer pipeline: {}", &pipeline_name);
        Ok(())
    }

    pub async fn sync_optional_pipelines(&self, settings: VideoStreamSettings) -> Result<()> {
        let hls_pipeline = self
            .make_hls_pipeline(HLS_PIPELINE, H264_ENCODING_PIPELINE, &settings)
            .await?;
        let hls_settings = &*(settings).hls;
        if hls_settings.enabled {
            hls_pipeline.pause().await?;
            hls_pipeline.play().await?;
        } else {
            hls_pipeline.stop().await?;
        }

        Ok(())
    }

    pub async fn start_video_recording_pipeline(&self, filename: &str) -> Result<()> {
        let settings = PrintNannySettings::new().await?;
        let camera = *settings.video_stream.camera;

        match self.delete_pipeline(H264_RECORDING_PIPELINE).await {
            Ok(_) => info!("Deleted existing pipeline={H264_RECORDING_PIPELINE}",),
            Err(e) => info!(
                "Failed to delete pipeline={H264_RECORDING_PIPELINE} error={}",
                e
            ),
        };

        let pipeline = self
            .make_recording_pipeline(
                H264_RECORDING_PIPELINE,
                H264_ENCODING_PIPELINE,
                filename,
                H264_SPLITMUXSINK,
                &camera,
            )
            .await?;
        pipeline.pause().await?;
        pipeline.play().await?;
        Ok(())
    }

    pub async fn stop_video_recording_pipeline(&self) -> Result<()> {
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(H264_RECORDING_PIPELINE);
        pipeline.emit_event_eos().await?;
        info!("Sent EOS signal to pipeline name={H264_RECORDING_PIPELINE}");
        pipeline.stop().await?;
        info!("Stopped pipeline name={H264_RECORDING_PIPELINE}");
        pipeline.delete().await?;
        info!("Deleted pipeline name={H264_RECORDING_PIPELINE}");
        Ok(())
    }

    pub async fn start_pipelines(&self) -> Result<()> {
        let mut settings = PrintNannySettings::new().await?;
        let old_video_stream_settings = settings.video_stream.clone();
        settings.video_stream = settings.video_stream.hotplug().await?;
        if settings.video_stream != old_video_stream_settings {
            warn!("start_pipelines detected a hotplug change in camera settings. Saving detected configuration");
            settings.save().await;
        }

        self.stop_pipelines().await?;

        let video_settings = settings.video_stream;

        let camera_pipeline = self
            .make_camera_pipeline(CAMERA_PIPELINE, &video_settings)
            .await?;

        let h264_pipeline = self
            .make_h264_encode_pipeline(H264_ENCODING_PIPELINE, CAMERA_PIPELINE, &video_settings)
            .await?;

        let rtp_pipeline = self
            .make_rtp_pipeline(RTP_PIPELINE, H264_ENCODING_PIPELINE, &video_settings)
            .await?;

        let inference_pipeline = self
            .make_inference_pipeline(INFERENCE_PIPELINE, CAMERA_PIPELINE, &video_settings)
            .await?;

        let bb_pipeline = self
            .make_bounding_box_pipeline(BB_PIPELINE, INFERENCE_PIPELINE, &video_settings)
            .await?;

        let df_pipeline = self
            .make_df_pipeline(DF_WINDOW_PIPELINE, INFERENCE_PIPELINE, &video_settings)
            .await?;

        let snapshot_pipeline = self
            .make_jpeg_snapshot_pipeline(SNAPSHOT_PIPELINE, CAMERA_PIPELINE, &video_settings)
            .await?;

        let mut pipelines = vec![
            camera_pipeline,
            h264_pipeline,
            rtp_pipeline,
            inference_pipeline,
            bb_pipeline,
            df_pipeline,
            snapshot_pipeline,
        ];

        let hls_settings = &*(video_settings).hls;

        if hls_settings.enabled {
            let hls_pipeline = self
                .make_hls_pipeline(HLS_PIPELINE, H264_ENCODING_PIPELINE, &video_settings)
                .await?;
            pipelines.push(hls_pipeline);
        }

        for pipeline in pipelines.iter() {
            info!("Setting pipeline name={} state=PAUSED", pipeline.name);
            pipeline.pause().await?;
        }

        for pipeline in pipelines {
            info!("Setting pipeline name={} state=PLAYING", pipeline.name);
            pipeline.play().await?;
        }

        Ok(())
    }

    pub async fn stop_pipelines(&self) -> Result<()> {
        warn!("Stopping gstreamer pipelines");
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let res = client.pipelines().await?;

        match res.response {
            gst_client::gstd_types::ResponseT::Properties(props) => {
                if let Some(nodes) = props.nodes {
                    for node in nodes {
                        let pipeline = client.pipeline(&node.name);
                        warn!("Stopping pipeline: {}", &node.name);
                        pipeline.stop().await?;
                        warn!("Deleting pipeline: {}", &node.name);
                        pipeline.delete().await?;
                    }
                }
            }
            _ => unimplemented!("Received invalid response to GET /pipelines"),
        };

        Ok(())
    }
}

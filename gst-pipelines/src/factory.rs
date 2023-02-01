use gst_client::reqwest;
use gst_client::GstClient;
use log::{error, info, warn};

use printnanny_settings::cam::VideoStreamSettings;
use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::printnanny_asyncapi_models::{CameraSettings, DetectionSettings};

use anyhow::Result;

const CAMERA_PIPELINE: &str = "camera";
const H264_PIPELINE: &str = "h264";
const RTP_PIPELINE: &str = "rtp";
const INFERENCE_PIPELINE: &str = "tflite_inference";
const BB_PIPELINE: &str = "bounding_boxes";
const DF_WINDOW_PIPELINE: &str = "df";
const SNAPSHOT_PIPELINE: &str = "snapshot";
const HLS_PIPELINE: &str = "hls";
const MP4_PIPELINE: &str = "mp4";

const GST_BUS_TIMEOUT: i32 = 6e+10 as i32; // 60 seconds (in nanoseconds)

pub struct PrintNannyPipelineFactory {
    pub address: String,
    pub port: i32,
    pub uri: String,
}

impl Default for PrintNannyPipelineFactory {
    fn default() -> Self {
        let address = "127.0.0.1".to_string();
        let port = 5002;
        let uri = Self::uri(&address, port);
        Self { address, port, uri }
    }
}

impl PrintNannyPipelineFactory {
    pub fn new(address: String, port: i32) -> Self {
        let uri = Self::uri(&address, port);
        Self { address, port, uri }
    }
    fn uri(address: &str, port: i32) -> String {
        format!("http://{}:{}", address, port)
    }

    fn to_interpipesrc_name(pipeline_name: &str) -> String {
        format!("{pipeline_name}_src")
    }

    fn to_interpipesink_name(pipeline_name: &str) -> String {
        format!("{pipeline_name}_sink")
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
                info!("Created camera pipeline: {:?}", result);
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

    async fn make_camera_pipeline(
        &self,
        pipeline_name: &str,
        camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        // imx219 sensor shows blue-tinted video feed when caps format/colorimetry are automatically negotiated
        // to reproduce this, run the following commands:

        // Normal colors:
        // GST_DEBUG=GST_CAPS:4 gst-launch-1.0 -vvv libcamerasrc ! 'video/x-raw,width=1280,height=720,format=YUY2' ! v4l2convert ! v4l2h264enc extra-controls="controls,repeat_sequence_header=1" ! h264parse ! 'video/x-h264,level=(string)4' ! rtph264pay ! udpsink host=localhost port=20001

        // Blue colors:
        // GST_DEBUG=GST_CAPS:4 gst-launch-1.0 -vvv libcamerasrc ! 'video/x-raw,width=1280,height=720' ! v4l2convert ! v4l2h264enc extra-controls="controls,repeat_sequence_header=1" ! h264parse ! 'video/x-h264,level=(string)4' ! rtph264pay ! udpsink host=localhost port=20001

        // So we manually specify the YUY2 format
        // NOTE this appears to be an interaction with the v4l2h264enc element, which forces upstream caps to YUY2

        let caps = match camera.device_name.contains("imx219") {
            true => format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d},format=YUY2",
                width = camera.width,
                height = camera.height,
                framerate_n = camera.framerate_n,
                framerate_d = camera.framerate_d
            ),
            false => format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d}",
                width = camera.width,
                height = camera.height,
                framerate_n = camera.framerate_n,
                framerate_d = camera.framerate_d
            ),
        };
        let description = format!(
            "libcamerasrc camera-name={camera_name} \
            ! capsfilter caps={caps} \
            ! v4l2convert \
            ! interpipesink name={interpipesink} forward-events=true forward-eos=true emit-signals=true sync=false",
            camera_name=camera.device_name,
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_jpeg_snapshot_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filesink_location: &str,
        _camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true max-buffers=3 leaky-type=1 \
            ! v4l2jpegenc ! multifilesink location={filesink_location} max-files=2",
            // width=camera.width,
            // height=camera.height,
            // format=camera.format,
            // framerate_n=camera.framerate_n,
            // framerate_d=camera.framerate_d,
            // colorimetry=camera.colorimetry
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_h264_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        _camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);
        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true \
            ! v4l2h264enc extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse \
            ! capssetter caps=video/x-h264,level=(string)4,profile=(string)high \
            ! interpipesink name={interpipesink} sync=false",
            // width=camera.width,
            // height=camera.height,
            // format=camera.format,
            // framerate_n=camera.framerate_n,
            // framerate_d=camera.framerate_d,
            // colorimetry=camera.colorimetry
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_rtp_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        port: i32,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

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
        hls_segments_location: &str,
        hls_playlist_location: &str,
        hls_playlist_root: &str,
        framerate_n: &i32,
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

        let target_duration = (60 / framerate_n) + 1; // v4l2-ctl --list-ctrls-menu -d 11 -> h264_i_frame_period default sends a key unit every 60 frames

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true format=3 \
            ! hlssink2 playlist-length=8 max-files=10 target-duration={target_duration} location={hls_segments_location} playlist-location={hls_playlist_location} playlist-root={hls_playlist_root} send-keyframe-requests=false");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_inference_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        tensor_width: i32,
        tensor_height: i32,
        tflite_model_file: &str,
        camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        let tensor_format = "RGB"; // model expects pixel data to be in RGB format
        let caps = match camera.device_name.contains("imx219") {
            true => format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d},format=YUY2",
                width = camera.width,
                height = camera.height,
                framerate_n = camera.framerate_n,
                framerate_d = camera.framerate_d
            ),
            false => format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d}",
                width = camera.width,
                height = camera.height,
                framerate_n = camera.framerate_n,
                framerate_d = camera.framerate_d
            ),
        };
        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true max-buffers=3 leaky-type=1 format=3 caps={caps} \
            ! v4l2convert ! videoscale ! videorate ! capsfilter caps=video/x-raw,format={tensor_format},width={tensor_width},height={tensor_height},framerate=0/1 \
            ! tensor_converter \
            ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1 \
            ! capsfilter caps=other/tensors,format=static \
            ! tensor_filter framework=tensorflow2-lite model={tflite_model_file} \
            ! interpipesink name={interpipesink} sync=false",
            // width=camera.width,
            // height=camera.height,
            // format=camera.format,
            // colorimetry=camera.colorimetry
        );

        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_bounding_box_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        camera: &CameraSettings,
        detection: &DetectionSettings,
        port: i32,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true \
            ! tensor_decoder name=bb_tensor_decoder mode=bounding_boxes option1=mobilenet-ssd-postprocess option2={tflite_label_file} option3=0:1:2:3,{nms_threshold} option4={video_width}:{video_height} option5={tensor_width}:{tensor_height} \
            ! capsfilter caps=video/x-raw,width={video_width},height={video_height} \
            ! v4l2convert \
            ! v4l2h264enc output-io-mode=mmap capture-io-mode=mmap extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse \
            ! capssetter caps=video/x-h264,level=(string)4,profile=(string)high \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
            ! udpsink port={port}
            ",
            nms_threshold=detection.nms_threshold,
            tflite_label_file=detection.label_file,
            tensor_height=detection.tensor_height,
            tensor_width=detection.tensor_width,
            // format=camera.format,
            video_width=camera.width,
            video_height=camera.height,

        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_df_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        nms_threshold: i32,
        nats_server_uri: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let nms_threshold = nms_threshold as f32 / 100_f32;

        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=true \
            ! tensor_decoder name=df_tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
            ! dataframe_agg filter-threshold={nms_threshold} output-type=json \
            ! nats_sink nats-address={nats_server_uri}");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_mp4_filesink_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filename: &str,
        filesink_name: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=true is-live=true allow-renegotiation=true \
            ! mp4mux ! filesink location={filename} name={filesink_name}");
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
        let snapshot_settings = *settings.snapshot;
        let camera = *settings.camera;
        let hls_settings = *settings.hls;

        let snapshot_pipeline = self
            .make_jpeg_snapshot_pipeline(
                SNAPSHOT_PIPELINE,
                CAMERA_PIPELINE,
                &snapshot_settings.path,
                &camera,
            )
            .await?;
        if snapshot_settings.enabled {
            snapshot_pipeline.pause().await?;
            snapshot_pipeline.play().await?;
        } else {
            snapshot_pipeline.stop().await?;
        }

        let hls_pipeline = self
            .make_hls_pipeline(
                HLS_PIPELINE,
                H264_PIPELINE,
                &hls_settings.segments,
                &hls_settings.playlist,
                &hls_settings.playlist_root,
                &camera.framerate_n,
            )
            .await?;
        if hls_settings.enabled {
            hls_pipeline.pause().await?;
            hls_pipeline.play().await?;
        } else {
            hls_pipeline.stop().await?;
        }

        Ok(())
    }

    pub async fn start_video_recording_pipeline(&self, filename: &str) -> Result<()> {
        let filesink_element_name = "mp4_filesink";
        let pipeline = self
            .make_mp4_filesink_pipeline(
                MP4_PIPELINE,
                H264_PIPELINE,
                filename,
                filesink_element_name,
            )
            .await?;

        // if pipeline was already created, ensure location property is set to filename
        let filesink_element = pipeline.element(filesink_element_name);
        filesink_element.set_property("location", filename).await?;
        info!("Updated Gstreamer element name={filesink_element_name} with property location={filename}");

        // set a filter for eos signal
        let bus = pipeline.bus();
        bus.set_filter("eos").await?;
        info!("Set filter for EOS events on {MP4_PIPELINE} pipeline bus");
        // set timeout for eos signal
        bus.set_timeout(GST_BUS_TIMEOUT).await?;
        info!("Set timeout ns={GST_BUS_TIMEOUT} for events on {MP4_PIPELINE} pipeline bus");

        pipeline.pause().await?;
        pipeline.play().await?;
        Ok(())
    }

    pub async fn stop_video_recording_pipeline(&self) -> Result<()> {
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(MP4_PIPELINE);
        info!("Sending EOS signal to pipeline name={MP4_PIPELINE}");
        let bus = pipeline.bus();
        pipeline.emit_event_eos().await?;
        // wait for eos signal to be emitted by pipeline bus
        match bus.read().await {
            Ok(res) => {
                info!(
                    "Event on pipeline name={MP4_PIPELINE} message bus event={:#?}",
                    res
                );
            }
            Err(e) => {
                error!(
                    "Error reading events on pipeline name={MP4_PIPELINE} error={}",
                    e
                )
            }
        };

        pipeline.stop().await?;
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

        let snapshot_settings = *settings.video_stream.snapshot;
        let camera = *settings.video_stream.camera;
        let hls_settings = *settings.video_stream.hls;
        let rtp_settings = *settings.video_stream.rtp;

        let detection_settings = *settings.video_stream.detection;

        let camera_pipeline = self.make_camera_pipeline(CAMERA_PIPELINE, &camera).await?;

        let h264_pipeline = self
            .make_h264_pipeline(H264_PIPELINE, CAMERA_PIPELINE, &camera)
            .await?;

        let rtp_pipeline = self
            .make_rtp_pipeline(RTP_PIPELINE, H264_PIPELINE, rtp_settings.video_udp_port)
            .await?;

        let inference_pipeline = self
            .make_inference_pipeline(
                INFERENCE_PIPELINE,
                CAMERA_PIPELINE,
                detection_settings.tensor_width,
                detection_settings.tensor_height,
                &detection_settings.model_file,
                &camera,
            )
            .await?;

        let bb_pipeline = self
            .make_bounding_box_pipeline(
                BB_PIPELINE,
                INFERENCE_PIPELINE,
                &camera,
                &detection_settings,
                rtp_settings.overlay_udp_port,
            )
            .await?;

        let df_pipeline = self
            .make_df_pipeline(
                DF_WINDOW_PIPELINE,
                INFERENCE_PIPELINE,
                detection_settings.nms_threshold,
                &detection_settings.nats_server_uri,
            )
            .await?;

        let mut pipelines = vec![
            camera_pipeline,
            h264_pipeline,
            rtp_pipeline,
            inference_pipeline,
            bb_pipeline,
            df_pipeline,
        ];

        if hls_settings.enabled {
            let hls_pipeline = self
                .make_hls_pipeline(
                    HLS_PIPELINE,
                    H264_PIPELINE,
                    &hls_settings.segments,
                    &hls_settings.playlist,
                    &hls_settings.playlist_root,
                    &camera.framerate_n,
                )
                .await?;
            pipelines.push(hls_pipeline);
        }

        if snapshot_settings.enabled {
            let snapshot_pipeline = self
                .make_jpeg_snapshot_pipeline(
                    SNAPSHOT_PIPELINE,
                    CAMERA_PIPELINE,
                    &snapshot_settings.path,
                    &camera,
                )
                .await?;
            pipelines.push(snapshot_pipeline);
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
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let res = client.pipelines().await?;

        match res.response {
            gst_client::gstd_types::ResponseT::Properties(props) => {
                if let Some(nodes) = props.nodes {
                    for node in nodes {
                        let pipeline = client.pipeline(&node.name);
                        info!("Stopping pipeline: {}", &node.name);
                        pipeline.stop().await?;
                        info!("Deleting pipeline: {}", &node.name);
                        pipeline.delete().await?;
                    }
                }
            }
            _ => unimplemented!("Received invalid response to GET /pipelines"),
        };

        Ok(())
    }
}

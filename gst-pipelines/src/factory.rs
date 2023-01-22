use gst_client::reqwest;
use gst_client::GstClient;
use log::{error, info};

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

pub struct PrintNannyPipelineFactory {
    pub address: String,
    pub port: i32,
    pub uri: String,
}

impl Default for PrintNannyPipelineFactory {
    fn default() -> Self {
        let address = "127.0.0.1".to_string();
        let port = 5001;
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
                    gst_client::Error::BadStatus(code) => match code {
                        reqwest::StatusCode::CONFLICT => {
                            info!("Pipeline with name={} already exists", pipeline_name);
                            Ok(())
                        }
                        _ => Err(e),
                    },
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
        let colorimetry = "bt709";
        let description = format!(
            "libcamerasrc camera-name={camera_name} \
            ! capsfilter caps=video/x-raw,width={width},height={height},framerate={framerate}/1,format={format},colorimetry={colorimetry} \
            ! interpipesink name={interpipesink} forward-events=true forward-eos=true emit-signals=true caps=video/x-raw,width={width},height={height},framerate={framerate}/1,format={format},colorimetry={colorimetry}",
            camera_name=camera.device_name,
            width=camera.width,
            height=camera.height,
            framerate=camera.framerate,
            format=camera.format
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_jpeg_snapshot_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filesink_location: &str,
        camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);
        let colorimetry = "bt709";

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=false max-buffers=3 leaky-type=1 caps=video/x-raw,width={width},height={height},format={format},colorimetry={colorimetry} \
            ! v4l2jpegenc ! multifilesink location={filesink_location} next-file=0",
            width=camera.width,
            height=camera.height,
            format=camera.format,
        );
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_h264_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        camera: &CameraSettings,
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        let colorimetry = "bt709";

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=false caps=video/x-raw,width={width},height={height},framerate={framerate}/1,format={format},colorimetry={colorimetry} \
            ! v4l2convert \
            ! v4l2h264enc min-force-key-unit-interval={framerate} extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse \
            ! capssetter caps=video/x-h264,colorimetry={colorimetry},level=(string)4 \
            ! interpipesink name={interpipesink} sync=false",
            width=camera.width,
            height=camera.height,
            format=camera.format,
            framerate=camera.framerate
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=false format=3 \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false format=3 \
            ! hlssink2 playlist-length=8 max-files=10 target-duration=1 location={hls_segments_location} playlist-location={hls_playlist_location} playlist-root={hls_playlist_root} send-keyframe-requests=false");
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
        let colorimetry = "bt709";

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false max-buffers=3 leaky-type=1 caps=video/x-raw,width={width},height={height},format={format},colorimetry={colorimetry} \
            ! videoconvert ! videoscale ! videorate ! capsfilter caps=video/x-raw,format={tensor_format},width={tensor_width},height={tensor_height},framerate=0/1 \
            ! tensor_converter \
            ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1 \
            ! capsfilter caps=other/tensors,format=static \
            ! tensor_filter framework=tensorflow2-lite model={tflite_model_file} \
            ! interpipesink name={interpipesink} sync=false",
            width=camera.width,
            height=camera.height,
            format=camera.format,
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false format=3 \
            ! tensor_decoder mode=bounding_boxes option1=mobilenet-ssd-postprocess option2={tflite_label_file} option3=0:1:2:3,{nms_threshold} option4={video_width}:{video_height} option5={tensor_width}:{tensor_height} \
            ! capsfilter caps=video/x-raw,width={video_width},height={video_height},format={format} \
            ! videoconvert \
            ! v4l2h264enc output-io-mode=mmap capture-io-mode=mmap extra-controls=controls,repeat_sequence_header=1 \
            ! h264parse \
            ! capsfilter caps=video/x-h264,level=(string)3,profile=(string)main \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
            ! udpsink port={port}
            ",
            nms_threshold=detection.nms_threshold,
            tflite_label_file=detection.label_file,
            tensor_height=detection.tensor_height,
            tensor_width=detection.tensor_width,
            format=camera.format,
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false \
            ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
            ! dataframe_agg filter-threshold={nms_threshold} output-type=json \
            ! nats_sink nats-address={nats_server_uri}");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_mp4_filesink_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filename: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let listen_to = Self::to_interpipesink_name(listen_to);

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false \
            ! mp4mux ! filesink location={filename}");
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

    pub async fn sync_optional_pipelines(&self) -> Result<()> {
        let settings = PrintNannySettings::new()?;
        let snapshot_settings = *settings.video_stream.snapshot;
        let camera = *settings.video_stream.camera;
        let hls_settings = *settings.video_stream.hls;

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
        let pipeline = self
            .make_mp4_filesink_pipeline(MP4_PIPELINE, H264_PIPELINE, filename)
            .await?;
        pipeline.pause().await?;
        pipeline.play().await?;
        Ok(())
    }

    pub async fn stop_video_recording_pipeline(&self) -> Result<()> {
        let client = GstClient::build(&self.uri).expect("Failed to build GstClient");
        let pipeline = client.pipeline(MP4_PIPELINE);
        pipeline.stop().await?;
        Ok(())
    }

    pub async fn start_pipelines(&self) -> Result<()> {
        let settings = PrintNannySettings::new()?;
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

        camera_pipeline.pause().await?;
        h264_pipeline.pause().await?;
        rtp_pipeline.pause().await?;
        inference_pipeline.pause().await?;
        bb_pipeline.pause().await?;
        df_pipeline.pause().await?;

        if hls_settings.enabled {
            let hls_pipeline = self
                .make_hls_pipeline(
                    HLS_PIPELINE,
                    H264_PIPELINE,
                    &hls_settings.segments,
                    &hls_settings.playlist,
                    &hls_settings.playlist_root,
                )
                .await?;
            hls_pipeline.pause().await?;
            hls_pipeline.play().await?;
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
            snapshot_pipeline.pause().await?;
            snapshot_pipeline.play().await?;
        }

        camera_pipeline.play().await?;
        h264_pipeline.play().await?;
        rtp_pipeline.play().await?;
        inference_pipeline.play().await?;
        bb_pipeline.play().await?;
        df_pipeline.play().await?;

        Ok(())
    }
}

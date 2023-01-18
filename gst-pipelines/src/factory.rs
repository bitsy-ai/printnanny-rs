use gst_client::reqwest;
use gst_client::GstClient;
use log::{error, info};

use printnanny_settings::printnanny::PrintNannySettings;
use printnanny_settings::printnanny_asyncapi_models::CameraSettings;

use anyhow::Result;

pub struct PrintNannyPipelineFactory {
    pub address: String,
    pub port: i32,
    pub uri: String,
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
            ! interpipesink name={interpipesink} sync=false",
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false max-buffers=2 leaky-type=2 \
            ! capsfilter caps=video/x-raw,width={width},height={height},format={format} \
            ! v4l2jpegenc ! multifilesink max-files=2 location={filesink_location}",
            width=camera.width,
            height=camera.height,
            format=camera.format
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


        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=false  \
            ! capsfilter caps=video/x-raw,width={width},height={height},framerate={framerate}/1,format={format},colorimetry={colorimetry} \
            ! v4l2convert \
            ! v4l2h264enc min-force-key-unit-interval={framerate} extra-controls='controls,repeat_sequence_header=1' \
            ! h264parse \
            ! capssetter caps='video/x-h264,colorimetry={colorimetry},level=(string)4' \
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

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=true accept-eos-event=false is-live=true allow-renegotiation=false \
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
    ) -> Result<gst_client::resources::Pipeline> {
        let listen_to = Self::to_interpipesink_name(listen_to);
        let interpipesrc = Self::to_interpipesrc_name(pipeline_name);
        let interpipesink = Self::to_interpipesink_name(pipeline_name);

        let format = "RGB"; // model expects pixel data to be in RGB format

        let description = format!("interpipesrc name={interpipesrc} listen-to={listen_to} accept-events=false accept-eos-event=false is-live=true allow-renegotiation=false max-buffers=2 leaky-type=2 \
            ! videoconvert ! videoscale ! capsfilter caps=video/x-raw,format={format},width={tensor_width},height={tensor_height} \
            ! tensor_converter \
            ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1 \
            ! capsfilter caps=other/tensors,format=static \
            ! tensor_filter framework=tensorflow2-lite model={tflite_model_file} \
            ! interpipesink name={interpipesink} sync=false");
        self.make_pipeline(pipeline_name, &description).await
    }

    async fn make_bounding_box_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        nms_threshold: i32,
        video_width: i32,
        video_height: i32,
        format: &str,
        tensor_width: i32,
        tensor_height: i32,
        tflite_label_file: &str,
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
            ");
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

    pub async fn start_pipelines(&self) -> Result<()> {
        let settings = PrintNannySettings::new()?;
        let snapshot_settings = *settings.video_stream.snapshot;
        let camera = *settings.video_stream.camera;
        let hls_settings = *settings.video_stream.hls;
        let rtp_settings = *settings.video_stream.rtp;

        let detection_settings = *settings.video_stream.detection;

        let camera_pipeline_name = "camera";
        let camera_pipeline = self
            .make_camera_pipeline(camera_pipeline_name, &camera)
            .await?;
        let h264_pipeline_name = "h264";
        let h264_pipeline = self
            .make_h264_pipeline(h264_pipeline_name, camera_pipeline_name, &camera)
            .await?;
        let rtp_pipeline_name = "rtp";
        let rtp_pipeline = self
            .make_rtp_pipeline(
                rtp_pipeline_name,
                h264_pipeline_name,
                rtp_settings.video_udp_port,
            )
            .await?;

        // let inference_pipeline_name = "tflite_inference";
        // let inference_pipeline = self
        //     .make_inference_pipeline(
        //         inference_pipeline_name,
        //         camera_pipeline_name,
        //         detection_settings.tensor_width,
        //         detection_settings.tensor_height,
        //         &detection_settings.model_file,
        //     )
        //     .await?;

        // let bb_pipeline_name = "bounding_boxes";
        // let bb_pipeline = self
        //     .make_bounding_box_pipeline(
        //         bb_pipeline_name,
        //         inference_pipeline_name,
        //         detection_settings.nms_threshold,
        //         camera.width,
        //         camera.height,
        //         camera.format,
        //         detection_settings.tensor_width,
        //         detection_settings.tensor_height,
        //         &detection_settings.label_file,
        //         rtp_settings.overlay_udp_port,
        //     )
        //     .await?;

        // let df_pipeline_name = "df";
        // let df_pipeline = self
        //     .make_df_pipeline(
        //         df_pipeline_name,
        //         inference_pipeline_name,
        //         detection_settings.nms_threshold,
        //         &detection_settings.nats_server_uri,
        //     )
        //     .await?;


        // if snapshot_settings.enabled {
        //     let snapshot_pipeline_name = "snapshot";
        //     let snapshot_pipeline = self
        //         .make_jpeg_snapshot_pipeline(
        //             snapshot_pipeline_name,
        //             camera_pipeline_name,
        //             &snapshot_settings.path,
        //             &camera
        //         )
        //         .await?;
        //     snapshot_pipeline.play().await?;
        // }


        // if hls_settings.enabled {
        //     let hls_pipeline_name = "hls";
        //     let hls_pipeline = self
        //         .make_hls_pipeline(
        //             hls_pipeline_name,
        //             h264_pipeline_name,
        //             &hls_settings.segments,
        //             &hls_settings.playlist,
        //             &hls_settings.playlist_root,
        //         )
        //         .await?;
        //     hls_pipeline.play().await?;
        // }

        camera_pipeline.pause().await?;
        h264_pipeline.pause().await?;
        rtp_pipeline.pause().await?;

        camera_pipeline.play().await?;
        h264_pipeline.play().await?;
        rtp_pipeline.play().await?;
        // inference_pipeline.play().await?;
        // bb_pipeline.play().await?;
        // df_pipeline.play().await?;

        Ok(())
    }
}

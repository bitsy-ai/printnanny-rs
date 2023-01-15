use gst_client::reqwest;
use gst_client::GstClient;
use log::info;

use printnanny_settings::cam::PrintNannyCameraSettings;
use printnanny_settings::{
    cam::CameraVideoSource, cam::VideoSource, printnanny::PrintNannySettings, SettingsFormat,
};

use anyhow::Result;

pub fn gst_client_address(args: &clap::ArgMatches) -> String {
    let address = args.value_of("http-address").unwrap();
    let port = args.value_of("http-port").unwrap();
    format!("http://{address}:{port}")
}

pub struct PrintNannyPipelineFactory {
    address: String,
    port: i32,
    client: GstClient,
}

impl PrintNannyPipelineFactory {
    pub fn new(address: String, port: i32) -> Self {
        let uri = Self::uri(&address, port);
        let client = GstClient::build(&uri).expect("Failed to build GstClient");

        Self {
            address,
            port,
            client,
        }
    }
    fn uri(address: &str, port: i32) -> String {
        format!("http://{}:{}", address, port)
    }

    async fn make_pipeline(
        &self,
        pipeline_name: &str,
        description: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let pipeline = self.client.pipeline(pipeline_name);
        match pipeline.create(description).await {
            Ok(result) => {
                info!("Created camera pipeline: {:?}", result);
                Ok(())
            }
            Err(e) => match e {
                gst_client::Error::BadStatus(code) => match code {
                    reqwest::StatusCode::CONFLICT => {
                        info!("Pipeline with name={} already exists", pipeline_name);
                        Ok(())
                    }
                    _ => Err(e),
                },
                _ => Err(e),
            },
        }?;
        Ok(pipeline)
    }

    async fn make_camera_pipeline(
        &self,
        camera_settings: &PrintNannyCameraSettings,
        pipeline_name: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let camera = match camera_settings.camera {
            VideoSource::CSI(camera) => camera,
            VideoSource::USB(camera) => camera,
            _ => unimplemented!(),
        };
        let description = format!(
            "libcamerasrc camera-name={camera_name} \
            ! capsfilter caps=video/x-raw,format=(string){pixel_format},width=(int){width},height=(int){height},framerate=(fraction){framerate}/1 \
            ! interpipesink name={pipeline_name} sync=false",
            camera_name=camera.device_name,
            pixel_format=camera.caps.format,
            width=camera.caps.width,
            height=camera.caps.height,
            framerate=settings.camera.video_framerate,
        );
        self.make_pipeline(pipeline_name, &description)
    }

    async fn make_jpeg_snapshot_pipeline(
        &self,
        pipeline_name: &str,
        listen_to: &str,
        filesink_location: &str,
    ) -> Result<gst_client::resources::Pipeline> {
        let description = format!("interpipesrc name={pipeline_name} listen-to={listen_to} accept-events=false accept-eos-event=false enable-sync=false allow-renegotiation=false num-buffers=1 \
            ! v4l2jpegenc ! multifilesink location=\"{filesink_location}\"");
        self.make_pipeline(pipeline_name, &description)
    }

    pub async fn start_pipelines(&self) -> Result<()> {
        let settings = PrintNannySettings::new()?;

        let camera_pipeline_name = "camera";
        let camera_pipeline = self
            .make_camera_pipeline(camera_pipeline_name, &settings.camera)
            .await?;
        camera_pipeline.play().await?;

        let snapshot_pipeline_name = "snapshot";
        let snapshot_pipeline = self
            .make_jpeg_snapshot_pipeline(
                &snapshot_pipeline_name,
                &camera_pipeline_name,
                &settings.camera.snapshot_location,
            )
            .await?;

        Ok(())
    }
}

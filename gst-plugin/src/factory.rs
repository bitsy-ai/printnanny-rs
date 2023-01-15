use gst_client::reqwest;
use gst_client::GstClient;
use log::info;

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
}

impl PrintNannyPipelineFactory {
    pub fn new(address: String, port: i32) -> Self {
        Self { address, port }
    }
    fn http_address(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }

    pub async fn start_pipelines(&self) -> Result<()> {
        let settings = PrintNannySettings::new()?;
        let camera = match &settings.camera.camera {
            VideoSource::CSI(camera) => camera,
            VideoSource::USB(camera) => camera,
            _ => unimplemented!(),
        };
        let client = GstClient::build(self.http_address())?;

        let pipeline_name = "camera";
        let camera_pipeline = client.pipeline(pipeline_name);

        match camera_pipeline.create(format!(
            "libcamerasrc camera-name={camera_name} \
            ! capsfilter caps=video/x-raw,format=(string){pixel_format},width=(int){width},height=(int){height},framerate=(fraction){framerate}/1 \
            ! interpipesink name={pipeline_name} sync=false",
            camera_name=camera.device_name,
            pixel_format=camera.caps.format,
            width=camera.caps.width,
            height=camera.caps.height,
            framerate=settings.camera.video_framerate,
        ))
        .await {
            Ok(result) => {
                info!("Created camera pipeline: {:?}", result);
                Ok(())
            },
            Err(e) => match e {
                gst_client::Error::BadStatus(code) => match code {
                    reqwest::StatusCode::CONFLICT => {
                        info!("Pipeline with name={} already exists", pipeline_name);
                        Ok(())
                    },
                    _ => Err(e)
                },
                _ => Err(e)
            }

        }?;

        camera_pipeline.play().await?;

        Ok(())
    }
}

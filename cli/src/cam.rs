use std::io;
use std::io::Write;

use anyhow::{Ok, Result};

use printnanny_gst_pipelines::factory::{PrintNannyPipelineFactory, MP4_RECORDING_PIPELINE};
use printnanny_settings::{cam::CameraVideoSource, SettingsFormat};

pub struct CameraCommand;

impl CameraCommand {
    async fn list(args: &clap::ArgMatches) -> Result<()> {
        let output = CameraVideoSource::from_libcamera_list().await?;
        let f: SettingsFormat = args.value_of_t("format").unwrap();

        let v = match f {
            SettingsFormat::Json => serde_json::to_vec_pretty(&output)?,
            SettingsFormat::Toml => toml::ser::to_vec(&output)?,
            _ => todo!(),
        };
        io::stdout().write_all(&v)?;

        Ok(())
    }

    async fn start_pipelines(args: &clap::ArgMatches) -> Result<()> {
        let address = args.value_of("http-address").unwrap();
        let port: i32 = args.value_of_t("http-port").unwrap();
        let factory = PrintNannyPipelineFactory::new(address.into(), port);
        factory.start_pipelines().await?;
        Ok(())
    }

    async fn stop_pipelines(args: &clap::ArgMatches) -> Result<()> {
        let address = args.value_of("http-address").unwrap();
        let port: i32 = args.value_of_t("http-port").unwrap();
        let factory = PrintNannyPipelineFactory::new(address.into(), port);
        factory.stop_pipelines().await?;
        Ok(())
    }

    async fn start_multifilesink_listener(args: &clap::ArgMatches) -> Result<()> {
        let address = args.value_of("http-address").unwrap();
        let port: i32 = args.value_of_t("http-port").unwrap();
        let pipeline = args.value_of("pipeline").unwrap();
        let factory = PrintNannyPipelineFactory::new(address.into(), port);
        factory
            .run_multifilesink_fragment_uploader(pipeline)
            .await?;
        Ok(())
    }

    pub async fn handle(args: &clap::ArgMatches) -> Result<()> {
        match args.subcommand() {
            Some(("list", args)) => Self::list(args).await,
            Some(("start-multifilesink-listener", args)) => {
                Self::start_multifilesink_listener(args).await
            }
            Some(("start-pipelines", args)) => Self::start_pipelines(args).await,
            Some(("stop-pipelines", args)) => Self::stop_pipelines(args).await,
            _ => unimplemented!(),
        }
    }
}

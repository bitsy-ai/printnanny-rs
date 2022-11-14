use anyhow::Result;
use printnanny_nats::cloud_worker::NatsCloudWorker;
use printnanny_services::config::PrintNannyConfig;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = PrintNannyConfig::new()?;
    // ensure pi, nats_app, nats_creds are provided
    config.try_check_license()?;

    let app = NatsCloudWorker::clap_command(None);
    let worker = NatsCloudWorker::new(&app.get_matches()).await?;
    worker.run().await?;
    Ok(())
}

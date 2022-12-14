use anyhow::Result;
use printnanny_nats::cloud_worker::NatsCloudWorker;

use printnanny_settings::cloud::PrintNannyCloudData;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let state = PrintNannyCloudData::new()?;
    // ensure pi, nats_app, nats_creds are provided
    state.try_check_cloud_data()?;

    let app = NatsCloudWorker::clap_command(None);
    let worker = NatsCloudWorker::new(&app.get_matches()).await?;
    worker.run().await?;
    Ok(())
}

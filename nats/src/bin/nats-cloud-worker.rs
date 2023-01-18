use anyhow::Result;
use printnanny_nats::cloud_worker::NatsCloudWorker;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsCloudWorker::clap_command(None);
    let worker = NatsCloudWorker::new(&app.get_matches()).await?;
    worker.run().await?;
    Ok(())
}

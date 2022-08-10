use anyhow::Result;
use env_logger;
use printnanny_nats::worker::NatsWorker;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsWorker::clap_command();
    let worker = NatsWorker::new(&app.get_matches()).await?;
    worker.run().await?;
    Ok(())
}

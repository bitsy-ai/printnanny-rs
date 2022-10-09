use anyhow::Result;
use printnanny_nats::message::NatsQcCommandRequest;
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsSubscriber::<NatsQcCommandRequest>::clap_command();
    let mut worker = NatsSubscriber::<NatsQcCommandRequest>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

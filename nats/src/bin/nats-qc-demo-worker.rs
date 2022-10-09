use anyhow::Result;
use printnanny_nats::message::NatsQcRequest;
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsSubscriber::<NatsQcRequest>::clap_command();
    let mut worker = NatsSubscriber::<NatsQcRequest>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

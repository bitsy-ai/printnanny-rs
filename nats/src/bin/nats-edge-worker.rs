use anyhow::Result;
use printnanny_nats::message::{NatsRequest, NatsResponse};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<NatsRequest, NatsResponse>::clap_command(None);
    let worker = NatsSubscriber::<NatsRequest, NatsResponse>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

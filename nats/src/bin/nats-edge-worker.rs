use anyhow::Result;
use printnanny_nats::message::{NatsReply, NatsRequest};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<NatsRequest, NatsReply>::clap_command(None);
    let worker = NatsSubscriber::<NatsRequest, NatsReply>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

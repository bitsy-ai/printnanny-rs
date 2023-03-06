use anyhow::Result;
use printnanny_nats::message_v2::{NatsEvent, NatsReply, NatsRequest};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::clap_command(None);
    let worker = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

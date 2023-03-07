use anyhow::Result;
use printnanny_nats_apps::event::NatsEvent;
use printnanny_nats_apps::request_reply::{NatsReply, NatsRequest};
use printnanny_nats_client::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::clap_command(None);
    let worker = NatsSubscriber::<NatsEvent, NatsRequest, NatsReply>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

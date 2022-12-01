use anyhow::Result;
use printnanny_nats::message_v2::{SettingsReply, SettingsRequest};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let app = NatsSubscriber::<SettingsRequest, SettingsReply>::clap_command(None);
    let worker = NatsSubscriber::<SettingsRequest, SettingsReply>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

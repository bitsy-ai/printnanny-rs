use anyhow::Result;
use env_logger;
use printnanny_nats::publisher::EventPublisher;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = EventPublisher::clap_command();
    let publisher = EventPublisher::new(&app.get_matches())?;
    publisher.run().await?;
    Ok(())
}

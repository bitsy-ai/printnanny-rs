use anyhow::Result;
use printnanny_nats::events::EventPublisher;

#[tokio::main]
async fn main() -> Result<()> {
    let app = EventPublisher::clap_command();
    let publisher = EventPublisher::new(&app.get_matches())?;
    publisher.run().await?;
    Ok(())
}

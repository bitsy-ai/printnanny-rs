use anyhow::Result;
use printnanny_nats::cloud_publisher::CloudEventPublisher;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = CloudEventPublisher::clap_command(None);
    let publisher = CloudEventPublisher::new(&app.get_matches()).await?;
    publisher.run().await?;
    Ok(())
}

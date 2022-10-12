use anyhow::Result;
use printnanny_nats::message::{QcCommandRequest, QcCommandResponse};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsSubscriber::<QcCommandRequest, QcCommandResponse>::clap_command(
        "nats-edge-worker",
    );
    let mut worker =
        NatsSubscriber::<QcCommandRequest, QcCommandResponse>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

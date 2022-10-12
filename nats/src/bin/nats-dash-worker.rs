use anyhow::Result;
use printnanny_nats::message::{NatsQcCommandRequest, NatsQcCommandResponse};
use printnanny_nats::subscriber::NatsSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = NatsSubscriber::<NatsQcCommandRequest, NatsQcCommandResponse>::clap_command(
        "nats-qc-worker",
    );
    let mut worker =
        NatsSubscriber::<NatsQcCommandRequest, NatsQcCommandResponse>::new(&app.get_matches());
    worker.run().await?;
    Ok(())
}

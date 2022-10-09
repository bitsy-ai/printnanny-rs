use anyhow::Result;
use printnanny_nats::worker::NatsWorker;
use printnanny_services::config::PrintNannyConfig;

use printnanny_nats::util::to_nats_command_subscribe_subject;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = PrintNannyConfig::new()?;
    // ensure pi, nats_app, nats_creds are provided
    config.try_check_license()?;

    // try_check_license guards the following properties set, so it's safe to unwrap here
    let pi = config.pi.unwrap();
    let nats_app = pi.nats_app.unwrap();

    let subject = to_nats_command_subscribe_subject(&pi.id);

    let app = NatsCloudWorker::clap_command();
    let mut worker = NatsCloudWorker::new(&app.get_matches()).await?;
    worker.run().await?;
    Ok(())
}

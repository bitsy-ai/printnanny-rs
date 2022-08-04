use anyhow::Result;
use printnanny_nats::worker;

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = worker::Worker::clap_command();
    let app_m = cmd.get_matches();
    let app = worker::Worker::new(app_m).await?;
    app.run().await?;
    Ok(())
}

use anyhow::Result;
use log::LevelFilter;
use printnanny_nats::publisher;

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = publisher::Worker::clap_command();
    let app_m = cmd.get_matches();
    let app = publisher::Worker::new(app_m);
    app.run().await?;
    Ok(())
}

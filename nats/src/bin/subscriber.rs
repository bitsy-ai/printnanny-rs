use anyhow::Result;
use printnanny_nats::subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = subscriber::Worker::clap_command();
    let app_m = cmd.get_matches();
    let app = subscriber::Worker::new(app_m);
    app.run().await?;
    Ok(())
}

use anyhow::Result;
use printnanny_nats::events::EventCommand;
#[tokio::main]
async fn main() -> Result<()> {
    let cmd = EventCommand::clap_command();
    let app_m = cmd.get_matches();
    let app = EventCommand::new(app_m);
    app.run().await?;
    Ok(())
}

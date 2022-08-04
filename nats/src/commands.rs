use anyhow::Result;
use async_process::Command;
use log::warn;
use printnanny_api_client::models;

pub async fn handle_pi_boot_command(
    cmd: models::polymorphic_pi_event::PiBootCommand,
) -> Result<()> {
    match cmd.event_type {
        models::PiBootCommandType::Reboot => {
            Command::new("reboot").output().await?;
        }
        models::PiBootCommandType::Shutdown => {
            Command::new("shutdown").output().await?;
        }
    };
    Ok(())
}

pub async fn handle_incoming(msg: models::PolymorphicPiEvent) -> Result<()> {
    match msg {
        models::PolymorphicPiEvent::PiBootCommand(command) => {
            handle_pi_boot_command(command).await?;
        }
        _ => warn!("No handler configured for msg={:?}", msg),
    };

    Ok(())
}

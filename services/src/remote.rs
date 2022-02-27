use super::config::PrintNannyConfig;
use anyhow::{Context, Result};
use async_process::Command as AsyncCommand;
use log::info;
use printnanny_api_client::models;

pub async fn run_playbook(
    event: models::PolymorphicEvent,
    config: PrintNannyConfig,
    dryrun: bool,
) -> Result<()> {
    let event_json =
        serde_json::to_string(&event).context(format!("Failed to serialize event {:?}", event))?;
    let event_name = match &event {
        models::PolymorphicEvent::WebRtcEvent(e) => e.event_name,
        _ => panic!("Failed to extract event_name from event {:?}", event),
    };

    let output = match dryrun {
        true => AsyncCommand::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection_name,
                event_name.to_string()
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .arg("--check")
            .output()
            .await
            .context(format!(
                "ansible-playbook command failed for event={:?}",
                event
            )),
        false => AsyncCommand::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection_name,
                event_name.to_string()
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .output()
            .await
            .context(format!(
                "ansible-playbook command failed for event={:?}",
                event
            )),
    }?;
    match output.status.success() {
        true => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("Success! event={:?} stdout={:?}", event, stdout);
            Ok(())
        }
        false => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            info!(
                "Command failed code={} event={:?} stdout={} stderr={}",
                output.status, event, stdout, stderr
            );
            Ok(())
        }
    }
}

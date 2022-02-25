use super::config::PrintNannyConfig;
use anyhow::Result;
use async_process::Command as AsyncCommand;
use log::info;
use printnanny_api_client::models;
use std::process::ExitStatus;

pub async fn handle_command(
    event: models::PolymorphicEvent,
    config: PrintNannyConfig,
    dryrun: bool,
) -> Result<()> {
    let event_json = serde_json::to_string(&event)?;
    let event_name = match event {
        models::PolymorphicEvent::WebRtcEvent(e) => e.event_name,
        _ => panic!("Failed to extract event_name from event {:?}", event),
    };

    let output = match dryrun {
        true => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection,
                event_name.to_string()
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .arg("--check")
            .output()
            .await
            .expect("ansible-playbook command failed"),
        false => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection,
                event_name.to_string()
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .output()
            .await
            .expect("ansible-playbook command failed"),
    };
    match output.status {
        ExitStatus::Success => {
            info!(
                "Success! event_type={} event_name={} stdout={}",
                event.event_type, event.event_name, output.stdout
            );
        }
        _ => {
            info!(
                "Command failed code={} event_type={} event_name={} stdout={} stderr={}",
                output.code, event.event_type, event.event_name, output.stdout, output.stderr
            );
        }
    }
}

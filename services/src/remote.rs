use super::config::PrintNannyConfig;
use anyhow::Result;
use log::info;
use printnanny_api_client::models;
use std::process::Command;

pub fn handle_event(
    event: models::PolymorphicEvent,
    config: PrintNannyConfig,
    dryrun: bool,
) -> Result<()> {
    let event_json = serde_json::to_string(&event)?;
    let event_name = match event {
        models::PolymorphicEvent::WebRtcEvent(e) => e.event_name,
        _ => panic!("Failed to extract event_name from event {:?}", event),
    };

    let mut cmd = match dryrun {
        true => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{:?}",
                config.ansible.collection, event_name
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .arg("--check")
            .spawn()
            .expect("ansible-playbook command failed"),
        false => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{:?}",
                config.ansible.collection, event_name
            ))
            .arg("-e")
            .arg(format!("'{}'", event_json))
            .spawn()
            .expect("ansible-playbook command failed"),
    };
    let ecode = cmd.wait().expect("ansible-playbook command failed to exit");
    info!("ansible-playbook exited with code {:?}", ecode.code());
    Ok(())
}

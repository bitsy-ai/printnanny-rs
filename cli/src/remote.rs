use anyhow::Result;
use log::info;
use printnanny_api_client::models;
use printnanny_services::config::PrintNannyConfig;
use std::process::Command;

pub fn handle_event(json_str: &str, config: PrintNannyConfig, dryrun: bool) -> Result<()> {
    let event: models::PolymorphicEvent =
        serde_json::from_str(json_str).expect("Failed to deserialize event");
    let cmd = match dryrun {
        true => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection, event.event_type
            ))
            .arg("--check")
            .spawn()
            .expect("ansible-playbook command failed"),
        false => Command::new(config.ansible.ansible_playbook())
            .arg(format!(
                "{}.events.{}",
                config.ansible.collection, event.event_type
            ))
            .spawn()
            .expect("ansible-playbook command failed"),
    };
    let ecode = cmd.wait().expect("ansible-playbook command failed to exit");
    info!("ansible-playbook exited with code {}", ecode.code());
    Ok(())
}

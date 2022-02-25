use super::config::PrintNannyConfig;
use async_process::Command as AsyncCommand;
use log::info;
use printnanny_api_client::models;

async fn run_playbook(event: models::PolymorphicEvent, config: PrintNannyConfig, dryrun: bool) {
    let event_json = serde_json::to_string(&event).expect("Failed to serialize event");
    let event_name = match &event {
        models::PolymorphicEvent::WebRtcEvent(e) => e.event_name,
        _ => panic!("Failed to extract event_name from event {:?}", event),
    };

    let output = match dryrun {
        true => AsyncCommand::new(config.ansible.ansible_playbook())
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
        false => AsyncCommand::new(config.ansible.ansible_playbook())
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
    match output.status.success() {
        true => {
            info!("Success! event={:?} stdout={:?}", event, output.stdout);
        }
        false => {
            info!(
                "Command failed code={} event={:?} stdout={:?} stderr={:?}",
                output.status, event, output.stdout, output.stderr
            );
        }
    }
}
pub async fn handle_command(
    event: models::PolymorphicEvent,
    config: PrintNannyConfig,
    dryrun: bool,
) -> () {
    run_playbook(event, config, dryrun).await;
}

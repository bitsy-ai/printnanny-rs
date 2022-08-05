use convert_case::{Case, Casing};

// returns full subject for event
// e.g.  pi_id 3, ShutdownCommand -> pi.3.shutdown.command
// e.g.  pi_id 3, ShutdownStarted -> pi.3.shutdown.started
pub fn to_nats_publish_subject(pi_id: &i32, prefix: &str, event_type: &str) -> String {
    // convert event_type from PascalCase to snake_case, then split on _, then re-join on .
    let s = event_type
        .to_case(Case::Snake)
        .split("_")
        .collect::<Vec<&str>>()
        .join(".");
    return format!("pi.{}.{}.{}", pi_id, prefix, &s);
}

// subscribe to commands with any subject prefix
pub fn to_nats_command_subscribe_subject(pi_id: &i32) -> String {
    return format!("pi.{}.*.*.command", pi_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_to_nats_publish_subject() {
        assert_eq!(
            to_nats_publish_subject(&3, "boot", "ShutdownCommand"),
            "pi.3.boot.shutdown.command"
        );
        assert_eq!(
            to_nats_publish_subject(&3, "boot", "ShutdownError"),
            "pi.3.boot.shutdown.error"
        );
        assert_eq!(
            to_nats_publish_subject(&3, "alert", "PrintQuality"),
            "pi.3.alert.print.quality"
        );
        assert_eq!(
            to_nats_publish_subject(&3, "alert", "PrintProgress"),
            "pi.3.alert.print.progress"
        )
    }

    #[test_log::test]
    fn test_to_nats_command_subscribe_subject() {
        assert_eq!(to_nats_command_subscribe_subject(&3), "pi.3.>.command");
    }
}

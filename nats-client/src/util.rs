// subscribe to commands with any subject prefix
pub fn to_nats_command_subscribe_subject(pi_id: &i32) -> String {
    format!("pi.{}.command.>", pi_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn test_to_nats_command_subscribe_subject() {
        assert_eq!(to_nats_command_subscribe_subject(&3), "pi.3.command.>");
    }
}

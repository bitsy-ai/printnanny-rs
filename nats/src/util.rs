use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use printnanny_services::error::CommandError;

// subscribe to commands with any subject prefix
pub fn to_nats_command_subscribe_subject(pi_id: &i32) -> String {
    return format!("pi.{}.command.>", pi_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn test_to_nats_command_subscribe_subject() {
        assert_eq!(to_nats_command_subscribe_subject(&3), "pi.3.command.>");
    }
}

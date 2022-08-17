// TODO
// can these values be reflected from serde rename macro?
//
pub const SUBJECT_COMMAND_BOOT: &str = "pi.{pi_id}.command.boot";
pub const SUBJECT_STATUS_BOOT: &str = "pi.{pi_id}.status.boot";

pub const SUBJECT_COMMAND_CAM: &str = "pi.{pi_id}.command.cam";
pub const SUBJECT_STATUS_CAM: &str = "pi.{pi_id}.status.cam";

pub const SUBJECT_COMMAND_SWUPDATE: &str = "pi.{pi_id}.command.swupdate";
pub const SUBJECT_STATUS_SWUPDATE: &str = "pi.{pi_id}.status.swupdate";

pub const SUBJECT_OCTOPRINT_SERVER: &str = "pi.{pi_id}.octoprint.server";
pub const SUBJECT_OCTOPRINT_PRINT_JOB: &str = "pi.{pi_id}.octoprint.print_job";
pub const SUBJECT_OCTOPRINT_PRINTER_STATUS: &str = "pi.{pi_id}.octoprint.printer";
pub const SUBJECT_OCTOPRINT_CLIENT: &str = "pi.{pi_id}.octoprint.client";

pub const SUBJECT_REPETIER: &str = "pi.{pi_id}.repetier";
pub const SUBJECT_MOONRAKER: &str = "pi.{pi_id}.moonraker";

macro_rules! format_nats_subject {
    (template: &str, pi_id: &str) => {
        format!(template)
    };
}

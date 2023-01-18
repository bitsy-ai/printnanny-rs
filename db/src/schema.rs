// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::sql_types::*;

    pis (id) {
        id -> Nullable<Integer>,
        last_boot -> Nullable<Text>,
        hostname -> Text,
        sbc -> SbcEnumMapping,
        created_dt -> Text,
        moonraker_api_url -> Text,
        mission_control_url -> Text,
        octoprint_url -> Text,
        swupdate_url -> Text,
        syncthing_url -> Text,
        preferred_dns -> PreferredDnsTypeMapping,
        octoprint_server_id -> Nullable<Integer>,
        system_info_id -> Nullable<Integer>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::sql_types::*;

    users (id) {
        id -> Nullable<Integer>,
        email -> Text,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::sql_types::*;

    video_recordings (id) {
        id -> Text,
        recording_status ->  RecordingStatusMapping,
        recording_start -> Nullable<BigInt>,
        recording_end -> Nullable<BigInt>,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_status ->RecordingStatusMapping,
        cloud_sync_start -> Nullable<BigInt>,
        cloud_sync_end -> Nullable<BigInt>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(pis, users, video_recordings,);

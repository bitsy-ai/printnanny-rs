// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    network_settings (id) {
        id -> Nullable<Integer>,
        updated_dt -> Text,
        preferred_dns -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    pi (id) {
        id -> Nullable<Integer>,
        last_boot -> Nullable<Text>,
        hostname -> Text,
        sbc -> Text,
        created_dt -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    pi_urls (id) {
        id -> Nullable<Integer>,
        moonraker_api -> Text,
        mission_control -> Text,
        octoprint -> Text,
        swupdate -> Text,
        syncthing -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    user (id) {
        id -> Nullable<Integer>,
        email -> Text,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    video_recordings (id) {
        id -> Text,
        recording_status -> Text,
        recording_start -> Nullable<BigInt>,
        recording_end -> Nullable<BigInt>,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_status -> Text,
        cloud_sync_start -> Nullable<BigInt>,
        cloud_sync_end -> Nullable<BigInt>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    network_settings,
    pi,
    pi_urls,
    user,
    video_recordings,
);

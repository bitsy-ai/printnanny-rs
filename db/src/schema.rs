// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    pi (id) {
        id -> Nullable<Integer>,
        last_boot -> Nullable<Text>,
        hostname -> Text,
        sbc -> Text,
        created_dt -> Text,
        moonraker_api_url -> Text,
        mission_control_url -> Text,
        octoprint_url -> Text,
        swupdate_url -> Text,
        syncthing_url -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    printnanny_cloud_api_config (user_id) {
        user_id -> Nullable<Integer>,
        base_url -> Text,
        bearer_access_token -> Nullable<Text>,
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
    pi,
    printnanny_cloud_api_config,
    user,
    video_recordings,
);

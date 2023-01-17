// @generated automatically by Diesel CLI.

diesel::table! {
    network_settings (id) {
        id -> Nullable<Integer>,
        updated_dt -> Text,
        preferred_dns -> Text,
    }
}

diesel::table! {
    pi (id) {
        id -> Nullable<Integer>,
        last_boot -> Text,
        hostname -> Text,
        sbc -> Text,
        created_dt -> Text,
    }
}

diesel::table! {
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
    user (id) {
        id -> Nullable<Integer>,
        email -> Text,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
    }
}

diesel::table! {
    video_recordings (id) {
        id -> Binary,
        recording_status -> Text,
        recording_start -> Nullable<Integer>,
        recording_end -> Nullable<Integer>,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_status -> Text,
        cloud_sync_start -> Nullable<Integer>,
        cloud_sync_end -> Nullable<Integer>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    network_settings,
    pi,
    pi_urls,
    user,
    video_recordings,
);

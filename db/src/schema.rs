// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;

    octoprint_servers (id) {
        id -> Integer,
        user_id -> Integer,
        pi_id -> Integer,
        octoprint_url -> Text,
        base_path -> Text,
        venv_path -> Text,
        pip_path -> Text,
        api_key -> Nullable<Text>,
        octoprint_version -> Nullable<Text>,
        pip_version -> Nullable<Text>,
        printnanny_plugin_version -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    pis (id) {
        id -> Integer,
        last_boot -> Nullable<Text>,
        hostname -> Text,
        created_dt -> Text,
        moonraker_api_url -> Text,
        mission_control_url -> Text,
        octoprint_url -> Text,
        swupdate_url -> Text,
        syncthing_url -> Text,
        preferred_dns -> Text,
        octoprint_server_id -> Nullable<Integer>,
        system_info_id -> Nullable<Integer>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    users (id) {
        id -> Nullable<Integer>,
        email -> Text,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

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
    octoprint_servers,
    pis,
    users,
    video_recordings,
);

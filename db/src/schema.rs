// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use diesel::sqlite::sql_types::*;

    nats_apps (id) {
        id -> Integer,
        app_name -> Text,
        pi_id -> Integer,
        organization_id -> Integer,
        organization_name -> Text,
        nats_server_uri -> Text,
        nats_ws_uri -> Text,
        mqtt_broker_host -> Text,
        mqtt_broker_port -> Integer,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use diesel::sqlite::sql_types::*;

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
    use diesel::sqlite::sql_types::*;

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
    use diesel::sqlite::sql_types::*;

    users (id) {
        id -> Nullable<Integer>,
        email -> Text,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use diesel::sqlite::sql_types::*;

    video_recordings (id) {
        id -> Text,
        recording_status -> Text,
        recording_start -> Nullable<TimestamptzSqlite>,
        recording_end -> Nullable<TimestamptzSqlite>,
        mp4_file_name -> Text,
        mp4_upload_url -> Nullable<Text>,
        mp4_download_url -> Nullable<Text>,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_status -> Text,
        cloud_sync_percent -> Nullable<Integer>,
        cloud_sync_start -> Nullable<TimestamptzSqlite>,
        cloud_sync_end -> Nullable<TimestamptzSqlite>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use diesel::sqlite::sql_types::*;

    webrtc_edge_servers (id) {
        id -> Integer,
        pi_id -> Integer,
        admin_secret -> Text,
        admin_port -> Integer,
        admin_url -> Text,
        api_token -> Text,
        api_domain -> Text,
        api_port -> Integer,
        pt -> Integer,
        rtp_domain -> Text,
        video_rtp_port -> Integer,
        data_rtp_port -> Integer,
        rtpmap -> Text,
        ws_port -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    nats_apps,
    octoprint_servers,
    pis,
    users,
    video_recordings,
    webrtc_edge_servers,
);

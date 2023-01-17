// @generated automatically by Diesel CLI.

diesel::table! {
    video_recordings (id) {
        id -> Binary,
        recording_start -> Integer,
        recording_end -> Nullable<Integer>,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_start -> Integer,
        cloud_sync_end -> Integer,
    }
}

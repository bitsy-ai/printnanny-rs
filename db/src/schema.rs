// @generated automatically by Diesel CLI.

diesel::table! {
    video_recordings (id) {
        id -> Binary,
        recording_start -> Nullable<Integer>,
        recording_end -> Nullable<Integer>,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        cloud_sync_start -> Nullable<Integer>,
        cloud_sync_end -> Nullable<Integer>,
    }
}

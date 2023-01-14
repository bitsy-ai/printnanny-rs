// @generated automatically by Diesel CLI.

diesel::table! {
    video_recordings (id) {
        id -> Binary,
        recording_file_name -> Text,
        gcode_file_name -> Nullable<Text>,
        ts -> Integer,
        backup_done -> Bool,
    }
}

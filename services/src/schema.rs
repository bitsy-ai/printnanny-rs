// @generated automatically by Diesel CLI.

diesel::table! {
    video_recordings (id) {
        id -> Binary,
        file_name -> Text,
        ts -> Integer,
        backup_done -> Bool,
    }
}

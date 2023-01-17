CREATE TABLE video_recordings (
  id blob(16) PRIMARY KEY NOT NULL,
  recording_start INTEGER NOT NULL,
  recording_end INTEGER,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name VARCHAR,
  cloud_sync_start INTEGER NOT NULL,
  cloud_sync_end INTEGER NOT NULL
)
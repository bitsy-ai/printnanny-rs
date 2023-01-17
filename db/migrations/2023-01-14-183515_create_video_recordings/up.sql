CREATE TABLE video_recordings (
  id blob(16) PRIMARY KEY NOT NULL,
  recording_start INTEGER,
  recording_end INTEGER,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name VARCHAR,
  cloud_sync_start INTEGER,
  cloud_sync_end INTEGER
)
CREATE TABLE video_recordings (
  id VARCHAR PRIMARY KEY NOT NULL,
  recording_status VARCHAR NOT NULL,
  recording_start INTEGER,
  recording_end INTEGER,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name VARCHAR,
  cloud_sync_status VARCHAR NOT NULL,
  cloud_sync_start INTEGER,
  cloud_sync_end INTEGER
)
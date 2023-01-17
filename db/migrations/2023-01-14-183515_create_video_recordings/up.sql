CREATE TABLE video_recordings (
  id VARCHAR PRIMARY KEY NOT NULL,
  recording_status VARCHAR NOT NULL,
  recording_start UNSIGNED BIG INT,
  recording_end UNSIGNED BIG INT,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name VARCHAR,
  cloud_sync_status VARCHAR NOT NULL,
  cloud_sync_start UNSIGNED BIG INT,
  cloud_sync_end UNSIGNED BIG INT
)
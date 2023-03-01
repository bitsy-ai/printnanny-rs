DROP TABLE IF EXISTS video_recordings;
CREATE TABLE video_recordings (
  id VARCHAR PRIMARY KEY NOT NULL,
  cloud_sync_done BOOLEAN NOT NULL,
  dir VARCHAR NOT NULL,
  recording_start DATETIME,
  recording_end DATETIME,
  gcode_file_name TEXT
)
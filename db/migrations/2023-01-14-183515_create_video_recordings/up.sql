CREATE TABLE video_recordings (
  id blob(16) PRIMARY KEY NOT NULL,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name VARCHAR,
  ts INTEGER NOT NULL,
  backup_done BOOLEAN NOT NULL DEFAULT FALSE
)
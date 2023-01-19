CREATE TABLE video_recordings (
  id VARCHAR PRIMARY KEY NOT NULL,
  recording_status TEXT CHECK(recording_status IN ('pending', 'inprogress', 'done')) NOT NULL,
  recording_start DATETIME,
  recording_end DATETIME,
  recording_file_name VARCHAR NOT NULL,
  gcode_file_name TEXT,
  cloud_sync_status TEXT CHECK(cloud_sync_status IN ('pending', 'inprogress', 'done')) NOT NULL,
  cloud_sync_start DATETIME,
  cloud_sync_end DATETIME
)
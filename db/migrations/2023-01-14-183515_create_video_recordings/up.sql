CREATE TABLE video_recordings (
  id VARCHAR PRIMARY KEY NOT NULL,
  deleted BOOLEAN NOT NULL,
  recording_status TEXT CHECK(recording_status IN ('pending', 'inprogress', 'done')) NOT NULL,
  recording_start DATETIME,
  recording_end DATETIME,
  mp4_file_name VARCHAR NOT NULL,
  mp4_upload_url VARCHAR,
  mp4_download_url VARCHAR,
  gcode_file_name TEXT,
  cloud_sync_status TEXT CHECK(cloud_sync_status IN ('pending', 'inprogress', 'done')) NOT NULL,
  cloud_sync_percent INTEGER,
  cloud_sync_start DATETIME,
  cloud_sync_end DATETIME
)
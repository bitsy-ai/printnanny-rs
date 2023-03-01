CREATE TABLE video_recording_parts (
  id VARCHAR PRIMARY KEY NOT NULL,
  part INTEGER NOT NULL,
  size BIGINT NOT NULL,
  deleted BOOLEAN NOT NULL,
  sync_start DATETIME,
  sync_end DATETIME,
  file_name VARCHAR NOT NULL,
  video_recording_id VARCHAR NOT NULL,
  FOREIGN KEY(video_recording_id) REFERENCES video_recordings(id)
)
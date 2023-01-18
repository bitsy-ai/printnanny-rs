CREATE TABLE webrtc_edge_servers (
  id INTEGER PRIMARY KEY NOT NULL,
  pi_id INTEGER NOT NULL,
  admin_secret VARCHAR NOT NULL,
  admin_port INTEGER NOT NULL,
  admin_url VARCHAR NOT NULL,
  api_token VARCHAR NOT NULL,
  api_domain VARCHAR NOT NULL,
  api_port INTEGER NOT NULL,
  pt INTEGER NOT NULL,
  rtp_domain VARCHAR NOT NULL,
  video_rtp_port INTEGER NOT NULL,
  data_rtp_port INTEGER NOT NULL,
  rtpmap VARCHAR NOT NULL,
  ws_port INTEGER NOT NULL
)
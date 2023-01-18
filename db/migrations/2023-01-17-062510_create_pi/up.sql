CREATE TABLE pis (
  id INTEGER PRIMARY KEY,
  last_boot VARCHAR,
  hostname VARCHAR NOT NULL,
  sbc TEXT CHECK(sbc IN ('rpi4')) NOT NULL,
  created_dt VARCHAR NOT NULL,
  moonraker_api_url VARCHAR NOT NULL,
  mission_control_url VARCHAR NOT NULL,
  octoprint_url VARCHAR NOT NULL,
  swupdate_url VARCHAR NOT NULL,
  syncthing_url VARCHAR NOT NULL,
  preferred_dns TEXT CHECK(preferred_dns IN ('multicast', 'tailscale')) NOT NULL,
  octoprint_server_id INTEGER,
  system_info_id INTEGER
)
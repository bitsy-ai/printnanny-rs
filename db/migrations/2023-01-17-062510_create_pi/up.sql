CREATE TABLE pi (
  id INTEGER PRIMARY KEY,
  last_boot VARCHAR,
  hostname VARCHAR NOT NULL,
  sbc VARCHAR NOT NULL,
  created_dt VARCHAR NOT NULL,
  moonraker_api_url VARCHAR NOT NULL,
  mission_control_url VARCHAR NOT NULL,
  octoprint_url VARCHAR NOT NULL,
  swupdate_url VARCHAR NOT NULL,
  syncthing_url VARCHAR NOT NULL
)
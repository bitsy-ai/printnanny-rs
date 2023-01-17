CREATE TABLE pi_urls (
  id INTEGER PRIMARY KEY,
  moonraker_api VARCHAR NOT NULL,
  mission_control VARCHAR NOT NULL,
  octoprint VARCHAR NOT NULL,
  swupdate VARCHAR NOT NULL,
  syncthing VARCHAR NOT NULL
)
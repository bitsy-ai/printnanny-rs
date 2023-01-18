CREATE TABLE octoprint_servers (
  id INTEGER PRIMARY KEY NOT NULL,
  user_id INTEGER NOT NULL,
  pi_id INTEGER NOT NULL,
  octoprint_url VARCHAR NOT NULL,
  base_path VARCHAR NOT NULL,
  venv_path VARCHAR NOT NULL,
  pip_path VARCHAR NOT NULL,
  api_key VARCHAR,
  octoprint_version VARCHAR,
  pip_version VARCHAR,
  printnanny_plugin_version VARCHAR

)
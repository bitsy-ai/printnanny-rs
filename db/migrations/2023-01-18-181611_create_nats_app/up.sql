CREATE TABLE nats_apps (
  id INTEGER PRIMARY KEY NOT NULL,
  app_name VARCHAR NOT NULL,
  pi_id INTEGER NOT NULL,
  organization_id INTEGER NOT NULL,
  organization_name VARCHAR NOT NULL,
  nats_server_uri VARCHAR NOT NULL,
  nats_ws_uri VARCHAR NOT NULL,
  mqtt_broker_host VARCHAR NOT NULL,
  mqtt_broker_port INTEGER NOT NULL
)
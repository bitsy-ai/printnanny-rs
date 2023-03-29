CREATE TABLE email_alert_settings (
  id INTEGER PRIMARY KEY NOT NULL,
  created_dt DATETIME NOT NULL,
  updated_dt DATETIME NOT NULL,
  progress_percent UNSIGNED INT NOT NULL,
  print_quality_enabled BOOLEAN NOT NULL,
  print_started_enabled BOOLEAN NOT NULL,
  print_done_enabled BOOLEAN NOT NULL,
  print_progress_enabled BOOLEAN NOT NULL,
  print_paused_enabled BOOLEAN NOT NULL,
  print_cancelled_enabled BOOLEAN NOT NULL
)
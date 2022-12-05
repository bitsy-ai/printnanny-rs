use std::path::PathBuf;

use log::warn;

pub async fn try_init_nats_client(
    nats_server_uri: &str,
    nats_creds: Option<PathBuf>,
    require_tls: bool,
) -> Result<async_nats::Client, std::io::Error> {
    match nats_creds {
        Some(nats_creds) => match nats_creds.exists() {
            true => {
                async_nats::ConnectOptions::with_credentials_file(nats_creds.clone())
                    .await?
                    .require_tls(require_tls)
                    .connect(nats_server_uri)
                    .await
            }
            false => {
                warn!(
                    "Failed to read {}. Initializing NATS client without credentials",
                    nats_creds.display()
                );
                async_nats::ConnectOptions::new()
                    .require_tls(require_tls)
                    .connect(nats_server_uri)
                    .await
            }
        },
        None => {
            async_nats::ConnectOptions::new()
                .require_tls(require_tls)
                .connect(nats_server_uri)
                .await
        }
    }
}

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrintNannyConfigError {
    #[error("Failed to handle invalid config value {value:?}")]
    InvalidValue { value: String },
    #[error("Refusing to overwrite existing keypair at {path:?}.")]
    KeypairExists { path: PathBuf },
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    OpenSSLError(#[from] openssl::error::ErrorStack),

    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    FigmentError(#[from] figment::error::Error),
}

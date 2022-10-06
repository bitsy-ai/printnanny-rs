use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializationError {
    #[error(transparent)]
    ArrowError {
        #[from]
        source: polars::error::ArrowError,
    },
    #[error(transparent)]
    PolarsError {
        #[from]
        source: polars::error::PolarsError,
    },
    #[error(transparent)]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error("Failed to unwrap BufWriter inner contents")]
    BufferError,
}

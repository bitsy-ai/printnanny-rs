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
    #[error("Failed to unwrap BufWriter inner contents")]
    BufferError,
}

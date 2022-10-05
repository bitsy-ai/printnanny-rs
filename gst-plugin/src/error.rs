use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializationError {
    #[error(transparent)]
    ArrowError {
        #[from]
        source: polars::error::ArrowError,
    },
    #[error("Failed to unwrap BufWriter inner contents")]
    BufferError,
}

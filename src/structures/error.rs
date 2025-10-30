use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("Transport closed")]
    Closed,

    #[error("Other: {0}")]
    Other(String),
}

pub type TResult<T> = Result<T, TransportError>;

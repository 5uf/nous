use thiserror::Error;

/// All errors that can occur within the Nous system.
#[derive(Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("object not found: {0}")]
    NotFound(String),

    #[error("corrupt object: expected {expected}, got {actual}")]
    Corrupt { expected: String, actual: String },

    #[error("invalid object id: {0}")]
    InvalidId(String),

    #[error("capability error: {0}")]
    Cap(String),

    #[error("http error: {0}")]
    Http(String),

    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the Nous workspace.
pub type Result<T> = std::result::Result<T, Error>;

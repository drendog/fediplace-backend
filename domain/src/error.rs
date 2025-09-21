use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Invalid tile coordinates: {0}")]
    InvalidTileCoordinates(String),

    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(String),

    #[error("Invalid pixel coordinates: {0}")]
    InvalidPixelCoordinates(String),

    #[error("Invalid color format: {0}")]
    InvalidColorFormat(String),

    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

pub type DomainResult<T> = Result<T, DomainError>;

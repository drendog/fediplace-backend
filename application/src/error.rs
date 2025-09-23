use std::io;
use thiserror::Error;

use domain::error::DomainError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("Invalid tile coordinates: {message}")]
    InvalidTileCoordinates { message: String },

    #[error("Invalid coordinates: {message}")]
    InvalidCoordinates { message: String },

    #[error("Invalid pixel coordinates: {message}")]
    InvalidPixelCoordinates { message: String },

    #[error("Invalid color format: {message}")]
    InvalidColorFormat { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Task error: {message}")]
    TaskError { message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("WebSocket error: {message}")]
    WebSocketError { message: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Cache error: {message}")]
    CacheError { message: String },

    #[error("Codec error: {message}")]
    CodecError { message: String },

    #[error("Internal server error")]
    InternalServerError,

    #[error("External service error: {message}")]
    ExternalServiceError { message: String },

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Email verification is required")]
    EmailNotVerified,

    #[error("Verification token not found or already used")]
    TokenNotFound,

    #[error("Verification token has expired")]
    TokenExpired,

    #[error("Insufficient credits: {message}")]
    InsufficientCredits { message: String },
}

pub type AppResult<T> = Result<T, AppError>;

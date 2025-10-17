use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::{debug, error};

use fedi_wplace_application::error::AppError;

pub struct HttpError(pub AppError);

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let app_error = &self.0;

        match app_error {
            AppError::Domain(_)
            | AppError::InvalidTileCoordinates { .. }
            | AppError::InvalidCoordinates { .. }
            | AppError::InvalidPixelCoordinates { .. }
            | AppError::InvalidColorFormat { .. }
            | AppError::ValidationError { .. }
            | AppError::JsonError(_)
            | AppError::WebSocketError { .. } => {
                debug!("Client error response generated: {}", app_error);
            }
            _ => {
                error!("Server error response generated: {}", app_error);
            }
        }

        let (status_code, message) = match app_error {
            AppError::Domain(_)
            | AppError::InvalidTileCoordinates { .. }
            | AppError::InvalidCoordinates { .. }
            | AppError::InvalidPixelCoordinates { .. }
            | AppError::InvalidColorFormat { .. }
            | AppError::WebSocketError { .. } => (StatusCode::BAD_REQUEST, app_error.to_string()),

            AppError::ValidationError { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, app_error.to_string())
            }

            AppError::ConfigError { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
            ),

            AppError::IoError(_) | AppError::TaskError { .. } | AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),

            AppError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON format".to_string()),

            AppError::DatabaseError { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),

            AppError::CacheError { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error".to_string())
            }

            AppError::CodecError { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Codec error".to_string())
            }

            AppError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".to_string(),
            ),

            AppError::ExternalServiceError { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "External service error".to_string(),
            ),

            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),

            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),

            AppError::NotFound { message } => (StatusCode::NOT_FOUND, message.clone()),

            AppError::EmailNotVerified => (
                StatusCode::FORBIDDEN,
                "Email verification is required".to_string(),
            ),

            AppError::TokenNotFound => (
                StatusCode::BAD_REQUEST,
                "Verification token not found or already used".to_string(),
            ),

            AppError::TokenExpired => (
                StatusCode::BAD_REQUEST,
                "Verification token has expired".to_string(),
            ),

            AppError::InsufficientCredits { message } => (StatusCode::FORBIDDEN, message.clone()),
        };

        let error_response = json!({
            "ok": false,
            "error": message,
            "status": status_code.as_u16()
        });

        (status_code, Json(error_response)).into_response()
    }
}

impl From<AppError> for HttpError {
    fn from(app_error: AppError) -> Self {
        HttpError(app_error)
    }
}

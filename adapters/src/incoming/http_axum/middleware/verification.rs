use axum::{extract::Request, middleware::Next, response::Response};
use axum_login::AuthSession;

use crate::incoming::http_axum::{auth::backend::AuthBackend, error_mapper::HttpError};
use fedi_wplace_application::error::AppError;

pub async fn require_email_verification(
    auth_session: AuthSession<AuthBackend>,
    request: Request,
    next: Next,
) -> Result<Response, HttpError> {
    let Some(user) = auth_session.user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    if user.email_verified_at.is_none() {
        return Err(HttpError(AppError::EmailNotVerified));
    }

    Ok(next.run(request).await)
}

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::common_responses::{
    InternalServerErrorResponse, RateLimitExceededResponse, UnauthorizedResponse,
    ValidationErrorResponse,
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_login::AuthSession;
use fedi_wplace_application::error::AppError;
use serde::Deserialize;
use serde_json::json;
#[cfg(feature = "docs")]
use utoipa;
use validator::Validate;

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::responses::{ApiResponseUser, ApiResponseValue};
use crate::{
    incoming::http_axum::{
        auth::backend::{AuthBackend, Credentials, User},
        dto::{
            requests::{LoginRequest, RegisterRequest, UpdateUsernameRequest},
            responses::{ApiResponse, UserResponse},
        },
        error_mapper::HttpError,
        handlers::auth_user_response::build_user_response,
    },
    shared::app_state::AppState,
};

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered and logged in successfully", body = ApiResponseUser,
         example = json!({
             "ok": true,
             "data": {
                 "id": "550e8400-e29b-41d4-a716-446655440000",
                 "email": "user@example.com",
                 "username": "johndoe",
                 "email_verified": false,
                 "available_charges": 25,
                 "charges_updated_at": "2023-01-01T12:00:00Z",
                 "charge_cooldown_seconds": 60,
                 "seconds_until_next_charge": 30,
                 "max_charges": 30,
                 "roles": []
             }
         })
        ),
        (status = 400, response = ValidationErrorResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Register a new user account",
    description = "Creates a new user account with email and password, then logs in the user with a session cookie."
))]
pub async fn register_handler(
    mut auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if let Err(e) = request.validate() {
        return Err(HttpError(AppError::ValidationError {
            message: format!("Validation failed: {}", e),
        }));
    }

    let user_public = state
        .auth_use_case
        .register_local(
            request.email.clone(),
            request.username.clone(),
            request.password,
        )
        .await?;

    let user = User::from(user_public.clone());
    auth_session
        .login(&user)
        .await
        .map_err(|_| HttpError(AppError::InternalServerError))?;

    let now = time::OffsetDateTime::now_utc();
    let user_response = build_user_response(user_public, &state, now).await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::<UserResponse> {
            ok: true,
            error: None,
            data: Some(user_response),
        }),
    ))
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in successfully", body = ApiResponseUser,
         example = json!({
             "ok": true,
             "data": {
                 "id": "550e8400-e29b-41d4-a716-446655440000",
                 "email": "user@example.com",
                 "username": "johndoe",
                 "email_verified": true,
                 "available_charges": 25,
                 "charges_updated_at": "2023-01-01T12:00:00Z",
                 "charge_cooldown_seconds": 60,
                 "seconds_until_next_charge": 30,
                 "max_charges": 30,
                 "roles": []
             }
         })
        ),
        (status = 400, response = ValidationErrorResponse),
        (status = 401, response = UnauthorizedResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Login with email and password",
    description = "Authenticates user credentials and creates a session cookie if successful."
))]
pub async fn login_handler(
    State(state): State<AppState>,
    mut auth_session: AuthSession<AuthBackend>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if let Err(e) = request.validate() {
        return Err(HttpError(AppError::ValidationError {
            message: e.to_string(),
        }));
    }

    let credentials = Credentials {
        email: request.email,
        password: request.password,
    };

    let user = auth_session
        .authenticate(credentials)
        .await
        .map_err(|_| HttpError(AppError::InternalServerError))?;

    let Some(user) = user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    auth_session
        .login(&user)
        .await
        .map_err(|_| HttpError(AppError::InternalServerError))?;

    let user_public = state.auth_use_case.me(user.id).await?;

    let now = time::OffsetDateTime::now_utc();
    let user_response = build_user_response(user_public, &state, now).await?;

    Ok(Json(ApiResponse::<UserResponse> {
        ok: true,
        error: None,
        data: Some(user_response),
    }))
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 204, description = "User logged out successfully"),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Logout current user",
    description = "Clears the user session and logs out the current user."
))]
pub async fn logout_handler(
    mut auth_session: AuthSession<AuthBackend>,
) -> Result<impl IntoResponse, HttpError> {
    auth_session
        .logout()
        .await
        .map_err(|_| HttpError(AppError::InternalServerError))?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "Current user information retrieved successfully", body = ApiResponseUser,
         example = json!({
             "ok": true,
             "data": {
                 "id": "550e8400-e29b-41d4-a716-446655440000",
                 "email": "user@example.com",
                 "username": "johndoe",
                 "email_verified": true,
                 "available_charges": 25,
                 "charges_updated_at": "2023-01-01T12:00:00Z",
                 "charge_cooldown_seconds": 60,
                 "seconds_until_next_charge": 30,
                 "max_charges": 30,
                 "roles": ["admin"],
                 "banned": false,
                 "ban_reason": null
             }
         })
        ),
        (status = 401, response = UnauthorizedResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Get current user information",
    description = "Returns information about the currently authenticated user."
))]
pub async fn me_handler(
    State(state): State<AppState>,
    auth_session: AuthSession<AuthBackend>,
) -> Result<impl IntoResponse, HttpError> {
    let Some(user) = auth_session.user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    let user_public = state.auth_use_case.me(user.id).await?;

    let now = time::OffsetDateTime::now_utc();
    let user_response = build_user_response(user_public, &state, now).await?;

    Ok(Json(ApiResponse::<UserResponse> {
        ok: true,
        error: None,
        data: Some(user_response),
    }))
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/auth/verify",
    params(
        ("token" = String, Query, description = "Email verification token")
    ),
    responses(
        (status = 200, description = "Email verified successfully", body = ApiResponseValue,
         example = json!({
             "ok": true,
             "data": {
                 "message": "Email verified successfully"
             }
         })
        ),
        (status = 400, description = "Invalid or expired token", body = ApiResponseValue,
         example = json!({
             "ok": false,
             "error": "Invalid or expired verification token"
         })
        ),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Verify email address",
    description = "Verifies a user's email address using the verification token sent to their email."
))]
pub async fn verify_email_handler(
    Query(query): Query<VerifyEmailQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, HttpError> {
    let _user_public = state.auth_use_case.verify_email(query.token).await?;

    Ok(Json(ApiResponse::<serde_json::Value> {
        ok: true,
        error: None,
        data: Some(json!({
            "message": "Email verified successfully"
        })),
    }))
}

#[cfg_attr(feature = "docs", utoipa::path(
    put,
    path = "/auth/username",
    request_body = UpdateUsernameRequest,
    responses(
        (status = 200, description = "Username updated successfully", body = ApiResponseUser,
         example = json!({
             "ok": true,
             "data": {
                 "id": "550e8400-e29b-41d4-a716-446655440000",
                 "email": "user@example.com",
                 "username": "new_username",
                 "email_verified": true,
                 "available_charges": 25,
                 "charges_updated_at": "2023-01-01T12:00:00Z",
                 "charge_cooldown_seconds": 60,
                 "seconds_until_next_charge": 30,
                 "max_charges": 30,
                 "roles": []
             }
         })
        ),
        (status = 400, response = ValidationErrorResponse),
        (status = 401, response = UnauthorizedResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "auth",
    summary = "Update username",
    description = "Updates the username for the currently authenticated user. Username must be unique and between 4-32 characters."
))]
pub async fn update_username_handler(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Json(request): Json<UpdateUsernameRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if let Err(e) = request.validate() {
        return Err(HttpError(AppError::ValidationError {
            message: format!("Validation failed: {}", e),
        }));
    }

    let Some(user) = auth_session.user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    let updated_user = state
        .auth_use_case
        .update_username(user.id, request.username)
        .await?;

    let now = time::OffsetDateTime::now_utc();
    let user_response = build_user_response(updated_user, &state, now).await?;

    Ok(Json(ApiResponse::<UserResponse> {
        ok: true,
        error: None,
        data: Some(user_response),
    }))
}

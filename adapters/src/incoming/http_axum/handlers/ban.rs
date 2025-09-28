use axum::{
    Json,
    extract::{Path, State},
};
use axum_login::AuthSession;
use time::format_description::well_known::Rfc3339;
use tracing::instrument;
use uuid::Uuid;

use crate::incoming::http_axum::{
    auth::backend::AuthBackend,
    dto::{
        requests::BanUserRequest,
        responses::{ApiResponse, BanResponse},
    },
    error_mapper::HttpError,
};
use crate::shared::app_state::AppState;
use domain::{auth::UserId, ban::Ban};
use fedi_wplace_application::error::AppError;

fn format_datetime(dt: time::OffsetDateTime) -> String {
    dt.format(&Rfc3339).unwrap_or_else(|_| dt.to_string())
}

fn parse_datetime_string(datetime_str: &str) -> Result<time::OffsetDateTime, AppError> {
    time::OffsetDateTime::parse(datetime_str, &Rfc3339).map_err(|_| AppError::ValidationError {
        message: "Invalid datetime format. Expected RFC3339 format (e.g., 2024-12-31T23:59:59Z)"
            .to_string(),
    })
}

impl From<Ban> for BanResponse {
    fn from(ban: Ban) -> Self {
        Self {
            id: *ban.id.as_uuid(),
            user_id: *ban.user_id.as_uuid(),
            banned_by_user_id: ban.banned_by_user_id.map(|id| *id.as_uuid()),
            reason: ban.reason,
            banned_at: format_datetime(ban.banned_at),
            expires_at: ban.expires_at.map(format_datetime),
            created_at: format_datetime(ban.created_at),
        }
    }
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/admin/users/{user_id}/ban",
    tag = "admin",
    request_body = BanUserRequest,
    responses(
        (status = 200, description = "User banned successfully", body = BanResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin role required)"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session" = [])
    )
))]
#[instrument(skip(auth_session, state))]
pub async fn ban_user(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<BanUserRequest>,
) -> Result<Json<ApiResponse<BanResponse>>, HttpError> {
    let current_user = auth_session.user.ok_or(HttpError(AppError::Unauthorized))?;

    if !current_user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let target_user_id = UserId::from_uuid(user_id);
    let banned_by_user_id = UserId::from_uuid(current_user.id);

    let expires_at = match request.expires_at {
        Some(expires_str) => Some(parse_datetime_string(&expires_str)?),
        None => None,
    };

    state
        .ban_use_case
        .ban_user(
            target_user_id.clone(),
            banned_by_user_id,
            request.reason,
            expires_at,
        )
        .await?;

    let ban = state
        .ban_use_case
        .check_user_ban_status(&target_user_id)
        .await?
        .ok_or(HttpError(AppError::InternalServerError))?;

    let response = ApiResponse::success_with_data(Some(BanResponse::from(ban)));
    Ok(Json(response))
}

#[cfg_attr(feature = "docs", utoipa::path(
    delete,
    path = "/admin/users/{user_id}/ban",
    tag = "admin",
    responses(
        (status = 200, description = "User unbanned successfully"),
        (status = 400, description = "User is not banned"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin role required)"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session" = [])
    )
))]
#[instrument(skip(auth_session, state))]
pub async fn unban_user(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, HttpError> {
    let current_user = auth_session.user.ok_or(HttpError(AppError::Unauthorized))?;

    if !current_user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let target_user_id = UserId::from_uuid(user_id);
    let unbanned_by = UserId::from_uuid(current_user.id);

    state
        .ban_use_case
        .unban_user(target_user_id, unbanned_by)
        .await?;

    let response = ApiResponse::success();
    Ok(Json(response))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/admin/bans",
    tag = "admin",
    responses(
        (status = 200, description = "List of active bans", body = Vec<BanResponse>),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin role required)"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session" = [])
    )
))]
#[instrument(skip(auth_session, state))]
pub async fn list_active_bans(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<BanResponse>>>, HttpError> {
    let current_user = auth_session.user.ok_or(HttpError(AppError::Unauthorized))?;

    if !current_user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let requesting_user_id = UserId::from_uuid(current_user.id);
    let bans = state
        .ban_use_case
        .get_active_bans(requesting_user_id)
        .await?;

    let ban_responses: Vec<BanResponse> = bans.into_iter().map(BanResponse::from).collect();
    let response = ApiResponse::success_with_data(Some(ban_responses));
    Ok(Json(response))
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/admin/users/{user_id}/ban",
    tag = "admin",
    responses(
        (status = 200, description = "User ban status", body = BanResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin role required)"),
        (status = 404, description = "User not banned or not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session" = [])
    )
))]
#[instrument(skip(auth_session, state))]
pub async fn get_user_ban_status(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<BanResponse>>, HttpError> {
    let current_user = auth_session.user.ok_or(HttpError(AppError::Unauthorized))?;

    if !current_user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let target_user_id = UserId::from_uuid(user_id);
    let ban = state
        .ban_use_case
        .check_user_ban_status(&target_user_id)
        .await?;

    match ban {
        Some(ban) => {
            let response = ApiResponse::success_with_data(Some(BanResponse::from(ban)));
            Ok(Json(response))
        }
        None => Err(HttpError(AppError::ValidationError {
            message: "User is not banned".to_string(),
        })),
    }
}

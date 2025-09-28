use axum::{
    Json,
    extract::{Path, State},
};
use axum_login::AuthSession;
use tracing::instrument;
use uuid::Uuid;

use crate::incoming::http_axum::{
    auth::backend::AuthBackend, dto::responses::UserResponse, error_mapper::HttpError,
    handlers::auth_user_response::build_user_response,
};
use crate::shared::app_state::AppState;
use fedi_wplace_application::error::AppError;

#[cfg_attr(feature = "docs", utoipa::path(
    put,
    path = "/admin/users/{user_id}/roles/{role_id}",
    tag = "admin",
    responses(
        (status = 200, description = "Role assigned successfully", body = UserResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin role required)"),
        (status = 404, description = "User or role not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session" = [])
    )
))]
#[instrument(skip(auth_session, state))]
pub async fn assign_role_to_user(
    auth_session: AuthSession<AuthBackend>,
    State(state): State<AppState>,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<UserResponse>, HttpError> {
    let current_user = auth_session.user.ok_or(HttpError(AppError::Unauthorized))?;

    if !current_user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let updated_user = state
        .admin_use_case
        .assign_role_to_user(user_id, role_id, current_user.id)
        .await?;

    let now = time::OffsetDateTime::now_utc();
    let response = build_user_response(updated_user, &state, now).await?;
    Ok(Json(response))
}

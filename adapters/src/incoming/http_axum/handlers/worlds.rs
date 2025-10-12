use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_login::AuthSession;
use uuid::Uuid;

use domain::world::WorldId;
use fedi_wplace_application::{error::AppError, world::service::WorldService};

use crate::{
    incoming::http_axum::{auth::backend::AuthBackend, error_mapper::HttpError},
    shared::app_state::AppState,
};

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct WorldResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct CreateWorldRequest {
    pub name: String,
}

pub async fn list_worlds(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorldResponse>>, HttpError> {
    let world_service: &WorldService = &state.world_service;
    let worlds = world_service.list_worlds().await.map_err(HttpError)?;

    let response: Vec<WorldResponse> = worlds
        .into_iter()
        .map(|world| WorldResponse {
            id: *world.id.as_uuid(),
            name: world.name,
            created_at: world.created_at.to_string(),
            updated_at: world.updated_at.to_string(),
        })
        .collect();

    Ok(Json(response))
}

pub async fn get_world_by_id(
    Path(world_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<WorldResponse>, HttpError> {
    let world_service: &WorldService = &state.world_service;
    let world_id = WorldId::from_uuid(world_id);

    let world = world_service
        .get_world_by_id(&world_id)
        .await
        .map_err(HttpError)?
        .ok_or(HttpError(AppError::NotFound {
            message: "World not found".to_string(),
        }))?;

    Ok(Json(WorldResponse {
        id: *world.id.as_uuid(),
        name: world.name,
        created_at: world.created_at.to_string(),
        updated_at: world.updated_at.to_string(),
    }))
}

pub async fn get_world_by_name(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<WorldResponse>, HttpError> {
    let world_service: &WorldService = &state.world_service;

    let world = world_service
        .get_world_by_name(&name)
        .await
        .map_err(HttpError)?
        .ok_or(HttpError(AppError::NotFound {
            message: "World not found".to_string(),
        }))?;

    Ok(Json(WorldResponse {
        id: *world.id.as_uuid(),
        name: world.name,
        created_at: world.created_at.to_string(),
        updated_at: world.updated_at.to_string(),
    }))
}

pub async fn create_world(
    State(state): State<AppState>,
    auth_session: AuthSession<AuthBackend>,
    Json(req): Json<CreateWorldRequest>,
) -> Result<(StatusCode, Json<WorldResponse>), HttpError> {
    let Some(user) = auth_session.user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    if !user.is_admin() {
        return Err(HttpError(AppError::Forbidden));
    }

    let world_service: &WorldService = &state.world_service;
    let world = world_service
        .create_world(req.name)
        .await
        .map_err(HttpError)?;

    Ok((
        StatusCode::CREATED,
        Json(WorldResponse {
            id: *world.id.as_uuid(),
            name: world.name,
            created_at: world.created_at.to_string(),
            updated_at: world.updated_at.to_string(),
        }),
    ))
}

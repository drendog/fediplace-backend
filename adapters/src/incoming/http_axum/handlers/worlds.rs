use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_login::AuthSession;
use uuid::Uuid;

#[cfg(feature = "docs")]
use utoipa::ToSchema;

use domain::world::{World, WorldId};
use fedi_wplace_application::{error::AppError, world::service::WorldService};

use crate::{
    incoming::http_axum::{auth::backend::AuthBackend, error_mapper::HttpError},
    shared::app_state::AppState,
};

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct PaletteEntry {
    pub id: i16,
    pub hex_color: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct WorldResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub tile_size: usize,
    pub pixel_size: usize,
    pub colors: Vec<PaletteEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct CreateWorldRequest {
    pub name: String,
}

async fn build_world_response(world: World, state: &AppState) -> Result<WorldResponse, AppError> {
    let palette_colors = state.world_service.get_palette_colors(&world.id).await?;

    let colors: Vec<PaletteEntry> = palette_colors
        .into_iter()
        .map(|palette_color| PaletteEntry {
            id: palette_color.palette_index,
            hex_color: palette_color.hex_color.0,
        })
        .collect();

    Ok(WorldResponse {
        id: *world.id.as_uuid(),
        name: world.name,
        created_at: world.created_at.to_string(),
        updated_at: world.updated_at.to_string(),
        tile_size: state.config.tiles.tile_size,
        pixel_size: state.config.tiles.pixel_size,
        colors,
    })
}

#[cfg_attr(
    feature = "docs",
    utoipa::path(
        get,
        path = "/worlds",
        responses(
            (status = 200, description = "List of all worlds with their configuration", body = Vec<WorldResponse>),
            (status = 500, description = "Internal server error")
        ),
        tag = "worlds"
    )
)]
pub async fn list_worlds(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorldResponse>>, HttpError> {
    let world_service: &WorldService = &state.world_service;
    let worlds = world_service.list_worlds().await.map_err(HttpError)?;

    let mut response = Vec::new();
    for world in worlds {
        let world_response = build_world_response(world, &state)
            .await
            .map_err(HttpError)?;
        response.push(world_response);
    }

    Ok(Json(response))
}

#[cfg_attr(
    feature = "docs",
    utoipa::path(
        get,
        path = "/worlds/{world_id}",
        params(
            ("world_id" = Uuid, Path, description = "World UUID")
        ),
        responses(
            (status = 200, description = "World configuration retrieved successfully", body = WorldResponse),
            (status = 404, description = "World not found"),
            (status = 500, description = "Internal server error")
        ),
        tag = "worlds"
    )
)]
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

    let response = build_world_response(world, &state)
        .await
        .map_err(HttpError)?;

    Ok(Json(response))
}

#[cfg_attr(
    feature = "docs",
    utoipa::path(
        get,
        path = "/worlds/by-name/{name}",
        params(
            ("name" = String, Path, description = "World name")
        ),
        responses(
            (status = 200, description = "World configuration retrieved successfully", body = WorldResponse),
            (status = 404, description = "World not found"),
            (status = 500, description = "Internal server error")
        ),
        tag = "worlds"
    )
)]
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

    let response = build_world_response(world, &state)
        .await
        .map_err(HttpError)?;

    Ok(Json(response))
}

#[cfg_attr(
    feature = "docs",
    utoipa::path(
        post,
        path = "/worlds",
        request_body = CreateWorldRequest,
        responses(
            (status = 201, description = "World created successfully", body = WorldResponse),
            (status = 401, description = "Not authenticated"),
            (status = 403, description = "Not authorized (admin only)"),
            (status = 500, description = "Internal server error")
        ),
        tag = "worlds"
    )
)]
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

    let response = build_world_response(world, &state)
        .await
        .map_err(HttpError)?;

    Ok((StatusCode::CREATED, Json(response)))
}

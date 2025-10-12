use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_login::AuthSession;
use axum_valid::Valid;
use domain::{auth::UserId, world::WorldId};
use uuid::Uuid;

use fedi_wplace_application::error::AppError;

use crate::incoming::http_axum::{
    auth::backend::AuthBackend,
    core::{
        etag,
        extractors::{IfNoneMatchHeader, extract_tile_coord},
    },
    dto::{
        requests::BatchPaintPixelsRequest,
        responses::{PaintPixelResponse, TileHeadersResponse, TileImageResponse},
    },
    error_mapper::HttpError,
};
use crate::shared::app_state::AppState;
use fedi_wplace_application::ports::incoming::tiles::{PaintPixelsUseCase, TilesQueryUseCase};

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::common_responses::{
    BadRequestResponse, ForbiddenResponse, InternalServerErrorResponse, NotAcceptableResponse,
    NotModifiedResponse, RateLimitExceededResponse, UnauthorizedResponse, ValidationErrorResponse,
};

fn is_not_modified(if_none_match: &IfNoneMatchHeader, etag: &str) -> bool {
    if let Some(typed_header) = if_none_match {
        if let Some(etag_parsed) = etag::parse(etag) {
            return !typed_header.precondition_passes(&etag_parsed);
        }
    }
    false
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/worlds/{world_id}/tiles/{x}/{y}",
    params(
        ("world_id" = Uuid, Path, description = "World ID"),
        ("x" = i32, Path, description = "Tile X coordinate"),
        ("y" = i32, Path, description = "Tile Y coordinate")
    ),
    responses(
        (status = 200, response = TileImageResponse),
        (status = 304, response = NotModifiedResponse),
        (status = 400, response = BadRequestResponse),
        (status = 406, response = NotAcceptableResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),

    tag = "tiles",
    summary = "Get tile image",
    description = "Retrieve a WebP image for the specified tile coordinates. Supports ETags for cache validation and conditional requests.",
    operation_id = "get_tile"
))]
pub async fn serve_tile(
    Path((world_id, x, y)): Path<(Uuid, i32, i32)>,
    _headers: HeaderMap,
    if_none_match: IfNoneMatchHeader,
    State(state): State<AppState>,
) -> Result<Response, HttpError> {
    let coord = extract_tile_coord(Path((x, y)))?;
    let world_id = WorldId::from_uuid(world_id);

    let tile_query_uc: &dyn TilesQueryUseCase = &*state.tiles_query_service;
    let tile_data = tile_query_uc
        .get_tile_webp(&world_id, coord)
        .await
        .map_err(HttpError)?;

    let etag = tile_data
        .etag
        .unwrap_or_else(|| etag::from_version(tile_data.version.as_u64()));

    if is_not_modified(&if_none_match, &etag) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    Ok(TileImageResponse {
        webp_data: tile_data.webp_data,
        etag,
        cache_control: state.config.tiles.http_cache_control.clone(),
    }
    .into_response())
}

#[cfg_attr(feature = "docs", utoipa::path(
    head,
    path = "/worlds/{world_id}/tiles/{x}/{y}",
    params(
        ("world_id" = Uuid, Path, description = "World ID"),
        ("x" = i32, Path, description = "Tile X coordinate"),
        ("y" = i32, Path, description = "Tile Y coordinate")
    ),
    responses(
        (status = 200, description = "Tile headers - HEAD", headers(
            ("ETag" = String),
            ("Cache-Control" = String),
            ("Content-Type" = String),
            ("RateLimit-Limit" = u32),
            ("RateLimit-Remaining" = u32),
            ("RateLimit-Reset" = u64)
        )),
        (status = 304, response = NotModifiedResponse),
        (status = 400, response = BadRequestResponse),
        (status = 406, response = NotAcceptableResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "tiles",
    summary = "Get tile headers",
    description = "Retrieve headers for the specified tile without the image data. Useful for cache validation and checking tile existence.",
    operation_id = "get_tile_head"
))]
pub async fn serve_tile_head(
    Path((world_id, x, y)): Path<(Uuid, i32, i32)>,
    _headers: HeaderMap,
    if_none_match: IfNoneMatchHeader,
    State(state): State<AppState>,
) -> Result<Response, HttpError> {
    let coord = extract_tile_coord(Path((x, y)))?;
    let world_id = WorldId::from_uuid(world_id);

    let tile_query_uc: &dyn TilesQueryUseCase = &*state.tiles_query_service;
    let tile_version = tile_query_uc
        .get_tile_version(&world_id, coord)
        .await
        .map_err(HttpError)?;

    let version_etag = etag::from_version(tile_version.as_u64());

    if is_not_modified(&if_none_match, &version_etag) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    Ok(TileHeadersResponse {
        etag: version_etag,
        cache_control: state.config.tiles.http_cache_control.clone(),
    }
    .into_response())
}

#[cfg_attr(feature = "docs", utoipa::path(
    post,
    path = "/worlds/{world_id}/tiles/{x}/{y}/pixels",
    params(
        ("world_id" = Uuid, Path, description = "World ID"),
        ("x" = i32, Path, description = "Tile X coordinate"),
        ("y" = i32, Path, description = "Tile Y coordinate")
    ),
    request_body = BatchPaintPixelsRequest,
    responses(
        (status = 200, body = PaintPixelResponse),
        (status = 202, body = PaintPixelResponse),
        (status = 400, response = BadRequestResponse),
        (status = 401, response = UnauthorizedResponse),
        (status = 403, response = ForbiddenResponse),
        (status = 422, response = ValidationErrorResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "painting",
    summary = "Paint multiple pixels",
    description = "Paint multiple pixels at once within a single tile. All pixels are painted atomically with a single version increment. Supports up to 1000 pixels per batch. Emits WebSocket tile-version for all painted pixels. Requires authentication.",
    operation_id = "paint_pixels_batch"
))]
pub async fn paint_pixels_batch(
    Path((world_id, x, y)): Path<(Uuid, i32, i32)>,
    State(state): State<AppState>,
    auth_session: AuthSession<AuthBackend>,
    Valid(Json(paint_req)): Valid<Json<BatchPaintPixelsRequest>>,
) -> Result<Json<PaintPixelResponse>, HttpError> {
    let Some(user) = auth_session.user else {
        return Err(HttpError(AppError::Unauthorized));
    };

    let tile_coord = extract_tile_coord(Path((x, y)))?;
    let world_id = WorldId::from_uuid(world_id);

    let pixels: Vec<_> = paint_req
        .pixels
        .iter()
        .map(|batch_pixel| (batch_pixel.pixel_coord(), batch_pixel.color_id()))
        .collect();

    let paint_uc: &dyn PaintPixelsUseCase = &*state.paint_pixels_service;
    let painting_result = paint_uc
        .paint_pixels_batch(&world_id, UserId::from_uuid(user.id), tile_coord, &pixels)
        .await
        .map_err(HttpError)?;

    Ok(Json(PaintPixelResponse {
        version: painting_result.new_version,
        write_id: painting_result.write_id,
    }))
}

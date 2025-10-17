use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use fedi_wplace_application::{error::AppError, ports::incoming::tiles::PixelInfoQueryUseCase};

use crate::incoming::http_axum::{dto::responses::PixelInfoResponse, error_mapper::HttpError};
use crate::shared::app_state::AppState;
use domain::{coords::GlobalCoord, world::WorldId};

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::common_responses::{
    BadRequestResponse, InternalServerErrorResponse, RateLimitExceededResponse,
};

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/worlds/{world_id}/pixels/{x}/{y}",
    responses(
        (status = 200, body = Option<PixelInfoResponse>, description = "Pixel information found", example = json!({"user_id": "12345", "username": "alice", "color_id": 15, "timestamp": "2025-09-10T12:34:56.789Z"})),
        (status = 400, response = BadRequestResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "pixel",
    summary = "Get pixel information",
    description = "Retrieve information about a specific pixel at the given global coordinates. Returns the user who last painted it, the color, and timestamp, and other info, or null if the pixel has never been painted.",
    operation_id = "get_pixel_info"
))]
pub async fn get_pixel_info(
    Path((world_id, x, y)): Path<(Uuid, i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<Option<PixelInfoResponse>>, HttpError> {
    let coord = GlobalCoord::new(x, y);
    coord.validate().map_err(|e| HttpError(AppError::from(e)))?;
    let world_id = WorldId::from_uuid(world_id);

    let pixel_info_uc: &dyn PixelInfoQueryUseCase = &*state.pixel_info_query_service;
    let app_pixel_info = pixel_info_uc
        .get_pixel_info(&world_id, coord)
        .await
        .map_err(HttpError)?;

    let pixel_info_response = app_pixel_info.map(|info| PixelInfoResponse {
        user_id: info.user_id,
        username: info.username,
        color_id: info.color_id,
        timestamp: info.timestamp.to_string(),
    });

    Ok(Json(pixel_info_response))
}

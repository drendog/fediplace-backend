use axum::{
    Json,
    extract::{Path, State},
};

use fedi_wplace_application::{error::AppError, ports::incoming::tiles::PixelHistoryQueryUseCase};

use crate::incoming::http_axum::{dto::responses::PixelHistoryEntry, error_mapper::HttpError};
use crate::shared::app_state::AppState;
use domain::coords::TileCoord;

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::common_responses::{
    BadRequestResponse, InternalServerErrorResponse, RateLimitExceededResponse,
};

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/tiles/{x}/{y}/history",
    responses(
        (status = 200, body = Vec<PixelHistoryEntry>),
        (status = 400, response = BadRequestResponse),
        (status = 429, response = RateLimitExceededResponse),
        (status = 500, response = InternalServerErrorResponse)
    ),
    tag = "tiles",
    summary = "Get tile pixel history",
    description = "Retrieve the history of all pixel changes for the specified tile coordinates. Returns an array of paint actions ordered by timestamp (newest first).",
    operation_id = "get_tile_history"
))]
pub async fn get_tile_history(
    Path((x, y)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PixelHistoryEntry>>, HttpError> {
    let coord = TileCoord::new(x, y);
    coord
        .validate_bounds()
        .map_err(|e| HttpError(AppError::from(e)))?;

    let history_uc: &dyn PixelHistoryQueryUseCase = &*state.pixel_history_query_service;
    let app_history_entries = history_uc
        .get_history_for_tile(coord)
        .await
        .map_err(HttpError)?;

    let history_entries: Vec<PixelHistoryEntry> = app_history_entries
        .into_iter()
        .map(|action| PixelHistoryEntry {
            user_id: action.user_id,
            username: action.username,
            px: action.pixel_x,
            py: action.pixel_y,
            color_id: action.color_id,
            timestamp: action.timestamp.to_string(),
        })
        .collect();

    Ok(Json(history_entries))
}

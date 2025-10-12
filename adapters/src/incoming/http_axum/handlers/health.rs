use axum::{Json, extract::State};

#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::responses::ApiResponseValue;
use crate::incoming::http_axum::{dto::responses::ApiResponse, error_mapper::HttpError};
use crate::shared::app_state::AppState;
use fedi_wplace_application::{
    error::AppError,
    ports::incoming::tiles::MetricsQueryUseCase,
};

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check successful with system metrics and config", body = ApiResponseValue,
         example = json!({
             "ok": true,
             "data": {
                 "metrics": {
                     "redis_cache_hit_rate": 0.85,
                     "active_tiles": 1042,
                     "flush_queue_size": 5
                 },
                 "config": {
                     "tile_size": 512,
                     "pixel_size": 1
                 }
             }
         })
        ),
        (status = 500, description = "Health check failed - system unhealthy", body = ApiResponseValue)
    ),
    tag = "system",
    summary = "System health check",
    description = "Get system health status including cache metrics, configuration, and performance statistics.",
    operation_id = "health_check"
))]
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, HttpError> {
    let default_world = state
        .world_service
        .get_default_world()
        .await
        .map_err(HttpError)?
        .ok_or_else(|| HttpError(AppError::NotFound {
            message: "Default world not found".to_string(),
        }))?;

    let metrics_uc: &dyn MetricsQueryUseCase = &*state.metrics_query_service;
    let metrics = metrics_uc
        .get_metrics(&default_world.id)
        .await
        .map_err(HttpError)?;

    Ok(Json(ApiResponse::success_with_data(Some(
        serde_json::json!({
            "metrics": metrics,
            "config": {
                "tile_size": state.config.tiles.tile_size,
                "pixel_size": state.config.tiles.pixel_size
            }
        }),
    ))))
}

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "docs")]
use utoipa::ToSchema;

use crate::incoming::http_axum::dto::responses::ApiResponse;
#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::responses::ApiResponseValue;
use crate::shared::app_state::AppState;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(
    feature = "docs",
    schema(
        description = "Canvas configuration with default world ID",
        example = json!({
            "default_world_id": "550e8400-e29b-41d4-a716-446655440000"
        })
    )
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfigResponse {
    pub default_world_id: Uuid,
}

#[cfg_attr(
    feature = "docs",
    utoipa::path(
        get,
        path = "/canvas/config",
        responses(
            (status = 200, description = "Canvas configuration retrieved successfully",
             body = ApiResponseValue,
             example = json!({
                 "ok": true,
                 "data": {
                     "default_world_id": "550e8400-e29b-41d4-a716-446655440000"
                 }
             })
            ),
            (status = 500, description = "Internal server error", body = ApiResponseValue)
        ),
        tag = "canvas"
    )
)]
pub async fn get_canvas_config(
    State(state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let canvas_config = match state.canvas_config_service.get_canvas_config().await {
        Ok(config) => config,
        Err(e) => {
            return Json(ApiResponse::<serde_json::Value> {
                ok: false,
                error: Some(format!("Failed to retrieve canvas config: {e}")),
                data: None,
            });
        }
    };

    let response = CanvasConfigResponse {
        default_world_id: *canvas_config.default_world_id.as_uuid(),
    };

    let response_data = match serde_json::to_value(response) {
        Ok(data) => Some(data),
        Err(_) => {
            return Json(ApiResponse::<serde_json::Value> {
                ok: false,
                error: Some("Failed to serialize canvas config data".to_string()),
                data: None,
            });
        }
    };

    Json(ApiResponse::success_with_data(response_data))
}

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

use crate::incoming::http_axum::dto::responses::ApiResponse;
#[cfg(feature = "docs")]
use crate::incoming::http_axum::dto::responses::ApiResponseValue;
use crate::shared::app_state::AppState;
use domain::color::RgbColor;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Color palette entry with ID and RGB values",
    example = json!({
        "id": 1,
        "color": {
            "r": 255,
            "g": 255,
            "b": 255
        }
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteEntry {
    pub id: u8,
    pub color: RgbColor,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Special color entry with ID and purpose description",
    example = json!({
        "id": 0,
        "purpose": "transparency"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialColorEntry {
    pub id: u8,
    pub purpose: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Complete color palette with regular paintable colors and special colors",
    example = json!({
        "regular_colors": [
            {
                "id": 1,
                "color": {
                    "r": 255,
                    "g": 255,
                    "b": 255
                }
            }
        ],
        "special_colors": [
            {
                "id": 0,
                "purpose": "transparency"
            }
        ]
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteResponse {
    pub regular_colors: Vec<PaletteEntry>,
    pub special_colors: Vec<SpecialColorEntry>,
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/palette",
    responses(
        (status = 200, description = "Color palette retrieved successfully",
         body = ApiResponseValue,
         example = json!({
             "ok": true,
             "data": {
                 "regular_colors": [
                     {
                         "id": 1,
                         "color": {
                             "r": 255,
                             "g": 255,
                             "b": 255
                         }
                     },
                     {
                         "id": 2,
                         "color": {
                             "r": 0,
                             "g": 0,
                             "b": 0
                         }
                     }
                 ],
                 "special_colors": [
                     {
                         "id": 0,
                         "purpose": "transparency"
                     }
                 ]
             }
         })
        ),
        (status = 500, description = "Internal server error", body = ApiResponseValue)
    ),
    tag = "palette"
))]

pub async fn get_palette(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let palette_config = &state.config.color_palette;

    let regular_colors: Vec<PaletteEntry> = palette_config
        .colors
        .iter()
        .enumerate()
        .map(|(index, color)| PaletteEntry {
            id: u8::try_from(index).unwrap_or(u8::MAX),
            color: *color,
        })
        .collect();

    let special_colors: Vec<SpecialColorEntry> = palette_config
        .get_special_colors_with_ids()
        .into_iter()
        .map(|(id, purpose)| SpecialColorEntry {
            id,
            purpose: purpose.clone(),
        })
        .collect();

    let palette_response = PaletteResponse {
        regular_colors,
        special_colors,
    };

    let response_data = match serde_json::to_value(palette_response) {
        Ok(data) => Some(data),
        Err(_) => {
            return Json(ApiResponse::<serde_json::Value> {
                ok: false,
                error: Some("Failed to serialize palette data".to_string()),
                data: None,
            });
        }
    };

    Json(ApiResponse::success_with_data(response_data))
}

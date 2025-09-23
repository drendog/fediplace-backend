use axum::{
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
    },
    response::{IntoResponse, Response},
};
use serde::Serialize;
#[cfg(feature = "docs")]
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Standard API response wrapper with success indicator, optional error message, and optional data payload",
    example = json!({
        "ok": true,
        "data": {
            "version": "1234567890",
            "snapped_coordinates": {
                "x": 128,
                "y": 256
            }
        }
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    #[must_use]
    pub fn success() -> Self {
        Self {
            ok: true,
            error: None,
            data: None,
        }
    }

    #[must_use]
    pub fn success_with_data(data: Option<T>) -> Self {
        Self {
            ok: true,
            error: None,
            data,
        }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Response for pixel painting operations with version and write ID",
    example = json!({
        "version": 1_234_567_890,
        "writeId": "550e8400-e29b-41d4-a716-446655440000"
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct PaintPixelResponse {
    #[cfg_attr(feature = "docs", schema(example = 1_234_567_890))]
    pub version: u64,
    #[cfg_attr(
        feature = "docs",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    #[serde(rename = "writeId")]
    pub write_id: String,
}

#[cfg_attr(feature = "docs", derive(ToResponse))]
#[cfg_attr(feature = "docs", response(
    description = "WebP tile image with cache headers",
    content_type = "image/webp",
    headers(
        ("ETag" = String),
        ("Cache-Control" = String),
        ("RateLimit-Limit" = u32),
        ("RateLimit-Remaining" = u32),
        ("RateLimit-Reset" = u64)
    )
))]
pub struct TileImageResponse {
    pub webp_data: Vec<u8>,
    pub etag: String,
    pub cache_control: String,
}

impl IntoResponse for TileImageResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));
        if let Ok(etag_value) = HeaderValue::from_str(&self.etag) {
            headers.insert(ETAG, etag_value);
        }
        if let Ok(cache_control_value) = HeaderValue::from_str(&self.cache_control) {
            headers.insert(CACHE_CONTROL, cache_control_value);
        }
        (headers, self.webp_data).into_response()
    }
}

pub struct TileHeadersResponse {
    pub etag: String,
    pub cache_control: String,
}

impl IntoResponse for TileHeadersResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(etag_value) = HeaderValue::from_str(&self.etag) {
            headers.insert(ETAG, etag_value);
        }
        if let Ok(cache_control_value) = HeaderValue::from_str(&self.cache_control) {
            headers.insert(CACHE_CONTROL, cache_control_value);
        }
        (headers, ()).into_response()
    }
}

pub type PaintOkEnvelope = ApiResponse<serde_json::Value>;

#[cfg(feature = "docs")]
#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(title = "ApiResponseValue")]
pub struct ApiResponseValue {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[cfg(feature = "docs")]
#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(title = "ApiResponseUser")]
pub struct ApiResponseUser {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<UserResponse>,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Role information",
    example = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "admin",
        "description": "Full access to all features and endpoints"
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct RoleResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    pub id: Uuid,
    #[cfg_attr(feature = "docs", schema(example = "admin"))]
    pub name: String,
    #[cfg_attr(feature = "docs", schema(example = "Full access to all features and endpoints"))]
    pub description: Option<String>,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "User data returned in authentication responses",
    example = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "email": "user@example.com",
        "username": "johndoe",
        "email_verified": true,
        "available_charges": 25,
        "charges_updated_at": "2023-01-01T12:00:00Z",
        "charge_cooldown_seconds": 60,
        "seconds_until_next_charge": 30,
        "max_charges": 30,
        "roles": ["admin"]
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    pub id: Uuid,
    #[cfg_attr(feature = "docs", schema(example = "user@example.com"))]
    pub email: String,
    #[cfg_attr(feature = "docs", schema(example = "johndoe"))]
    pub username: String,
    #[cfg_attr(feature = "docs", schema(example = true))]
    pub email_verified: bool,
    #[cfg_attr(feature = "docs", schema(example = 25))]
    pub available_charges: i32,
    #[cfg_attr(feature = "docs", schema(example = "2023-01-01T12:00:00Z"))]
    pub charges_updated_at: String,
    #[cfg_attr(feature = "docs", schema(example = 60))]
    pub charge_cooldown_seconds: i32,
    #[cfg_attr(feature = "docs", schema(example = 30))]
    pub seconds_until_next_charge: i64,
    #[cfg_attr(feature = "docs", schema(example = 30))]
    pub max_charges: i32,
    #[cfg_attr(feature = "docs", schema(example = json!(["admin"])))]
    pub roles: Vec<String>,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Single paint action in the tile history",
    example = json!({
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "username": "johndoe",
        "px": 128,
        "py": 64,
        "color_id": 5,
        "timestamp": "2023-01-01T12:00:00Z"
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct PixelHistoryEntry {
    #[cfg_attr(
        feature = "docs",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    pub user_id: Uuid,
    #[cfg_attr(feature = "docs", schema(example = "johndoe"))]
    pub username: String,
    #[cfg_attr(feature = "docs", schema(example = 128))]
    pub px: usize,
    #[cfg_attr(feature = "docs", schema(example = 64))]
    pub py: usize,
    #[cfg_attr(feature = "docs", schema(example = 5))]
    pub color_id: u8,
    #[cfg_attr(feature = "docs", schema(example = "2023-01-01T12:00:00Z"))]
    pub timestamp: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Information about a specific pixel",
    example = json!({
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "username": "johndoe",
        "color_id": 5,
        "timestamp": "2023-01-01T12:00:00Z"
    })
))]
#[derive(Debug, Clone, Serialize)]
pub struct PixelInfoResponse {
    #[cfg_attr(
        feature = "docs",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    pub user_id: Uuid,
    #[cfg_attr(feature = "docs", schema(example = "johndoe"))]
    pub username: String,
    #[cfg_attr(feature = "docs", schema(example = 5))]
    pub color_id: u8,
    #[cfg_attr(feature = "docs", schema(example = "2023-01-01T12:00:00Z"))]
    pub timestamp: String,
}

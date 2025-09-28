use crate::incoming::http_axum::{auth, dto, handlers};
use crate::incoming::ws_axum::{
    endpoint,
    protocol::{ClientMessage, RejectedTile, WSMessage},
};
use auth::oauth_google::AuthRequest;
use domain::{
    color::RgbColor,
    coords::{PixelCoord, TileCoord},
    tile::TileVersion,
};
use dto::common_responses::{
    BadRequestResponse, ForbiddenResponse, InternalServerErrorResponse, NotAcceptableResponse,
    NotModifiedResponse, RateLimitExceededResponse, UnauthorizedResponse, ValidationErrorResponse,
};
use dto::requests::{
    BatchPaintPixelsRequest, BatchPixelPaint, BanUserRequest, LoginRequest, PaintRequest, RegisterRequest,
    UpdateUsernameRequest,
};
#[cfg(feature = "docs")]
use dto::responses::{ApiResponseUser, ApiResponseValue};
use dto::responses::{
    BanResponse, PaintOkEnvelope, PaintPixelResponse, PixelHistoryEntry, PixelInfoResponse, TileImageResponse, UserResponse,
};
use handlers::palette::{PaletteEntry, PaletteResponse, SpecialColorEntry};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::tiles::serve_tile,
        handlers::tiles::serve_tile_head,
        handlers::tiles::paint_pixels_batch,
        handlers::palette::get_palette,
        handlers::pixel_info::get_pixel_info,
        handlers::health::health_check,
        handlers::auth::register_handler,
        handlers::auth::login_handler,
        handlers::auth::logout_handler,
        handlers::auth::me_handler,
        handlers::auth::update_username_handler,
        handlers::admin::assign_role_to_user,
        handlers::ban::ban_user,
        handlers::ban::unban_user,
        handlers::ban::list_active_bans,
        handlers::ban::get_user_ban_status,
        auth::oauth_google::google_auth_start,
        auth::oauth_google::google_auth_callback,
        endpoint::websocket_handler,
    ),
    components(
        schemas(
            PaintRequest,
            BatchPaintPixelsRequest,
            BatchPixelPaint,
            ApiResponseValue,
            ApiResponseUser,
            PaintOkEnvelope,
            PaintPixelResponse,
            PaletteEntry,
            PaletteResponse,
            SpecialColorEntry,
            RegisterRequest,
            LoginRequest,
            UpdateUsernameRequest,
            BanUserRequest,
            AuthRequest,
            UserResponse,
            BanResponse,
            PixelHistoryEntry,
            PixelInfoResponse,
            RgbColor,
            TileCoord,
            PixelCoord,
            TileVersion,
            WSMessage,
            ClientMessage,
            RejectedTile
        ),
        responses(
            TileImageResponse,
            NotModifiedResponse,
            BadRequestResponse,
            NotAcceptableResponse,
            RateLimitExceededResponse,
            InternalServerErrorResponse,
            UnauthorizedResponse,
            ForbiddenResponse,
            ValidationErrorResponse
        )
    ),
    tags(
        (name = "tiles", description = "Tile management operations - serve WebP tile images with caching and rate limiting"),
        (name = "painting", description = "Pixel painting operations - place pixels on tiles with rate limiting and backoff guidance"),
        (name = "palette", description = "Color palette management - retrieve available colors for pixel painting"),
        (name = "pixel", description = "Pixel information operations - retrieve metadata about individual pixels"),
        (name = "auth", description = "Authentication and user management - register, login, logout, and user profile operations"),
        (name = "admin", description = "Admin operations - role management and user administration (requires admin privileges)"),
        (name = "system", description = "System health and status monitoring"),
        (name = "websocket", description = "Real-time WebSocket protocol for collaborative pixel painting. Supports tile subscriptions, live updates, configurable IP-based limits, FIFO eviction policy, and rate limiting for both connection upgrades and individual messages.")
    ),
    info(
        title = "FediPlace Backend API",
        description = "A collaborative pixel painting server similar to r/place. Features tile-based rendering with WebP compression, real-time WebSocket updates, 3-tier caching architecture, and rate limiting with rate limit headers (RateLimit-Limit, RateLimit-Remaining, RateLimit-Reset, Retry-After).",
        contact(
            name = "FediPlace",
        ),
    ),
    servers(
        (url = "http://localhost:8000", description = "Development server"),
    )
)]
pub struct ApiDoc;

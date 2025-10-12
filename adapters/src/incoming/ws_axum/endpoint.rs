use axum::{
    body::Body,
    extract::{ConnectInfo, State, WebSocketUpgrade},
    http::{HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

use crate::incoming::http_axum::middleware::rate_limit::RateLimitResult;
use crate::shared::app_state::AppState;

use super::{handler::ConnectionHandler, ip_utils::extract_client_ip};

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/live",
    responses(
        (status = 101, description = "WebSocket connection established for real-time updates"),
        (status = 400, description = "Bad Request - WebSocket upgrade failed"),
        (status = 429, description = "Rate limit exceeded - WebSocket upgrade denied",
         headers(
             ("RateLimit-Limit" = u32, description = "Maximum WebSocket connections allowed per time window"),
             ("RateLimit-Remaining" = u32, description = "Connections remaining (0 when rate limited)"),
             ("RateLimit-Reset" = u64, description = "Unix timestamp when rate limit window resets"),
             ("Retry-After" = u64, description = "Suggested retry delay in seconds")
         )
        ),
        (status = 500, description = "Internal server error")
    ),
    tag = "websocket",
    summary = "Establish WebSocket connection for real-time collaboration",
    description = r"
Upgrades HTTP connection to WebSocket for real-time pixel updates and tile subscriptions.

## Protocol Overview
The WebSocket connection enables bidirectional communication between client and server for real-time collaborative pixel painting.

## Rate Limiting
- **Connection Upgrades**: WebSocket upgrade requests are rate limited per IP
- **Message Rate Limiting**: Individual WebSocket messages are rate limited after connection
- **429 Responses**: Failed upgrades return rate limit headers (RateLimit-Limit, RateLimit-Remaining, RateLimit-Reset, Retry-After)
- **Error Messages**: Rate limit violations within WebSocket connections receive structured error messages with rate limit details

## Subscription System (Configurable Policy)
- **Per-IP Limits**: Each IP address can subscribe to a configurable maximum number of tiles (default: 64 tiles)
- **FIFO Eviction**: When limit is exceeded, oldest subscriptions are automatically removed
- **TTL Management**: Subscriptions expire after a configurable timeout (default: 45 seconds of inactivity)
- **Heartbeat**: Send ping messages at configurable intervals (default: every 15 seconds) to keep subscriptions alive

All policy limits are configurable via environment variables or config.toml settings.

## Connection Flow
1. Client establishes WebSocket connection to `/live` (subject to rate limiting)
2. Client sends `subscribe` message with tile coordinates (subject to message rate limiting)
3. Server responds with `subscription-ack` confirming accepted/rejected tiles
4. Server broadcasts `tile-version` messages when tiles change on subscribed tiles
5. Client sends `ping` messages to maintain connection (subject to message rate limiting)
6. Client can `unsubscribe` from tiles when no longer needed

## Client Message Types
- `subscribe`: Subscribe to tiles for real-time updates
- `unsubscribe`: Unsubscribe from tiles
- `ping`: Heartbeat to keep connection alive

## Server Message Types
- `subscription-ack`: Confirmation of subscription request
- `tile-version`: Tile version change notification
- `error`: Error message (including rate limit violations)
- `subscription-confirmed`/`unsubscription-confirmed`: Operation confirmations

See the WebSocket message schemas for detailed message formats and examples.
    ",
    operation_id = "websocket_connect"
))]
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> Response {
    let client_ip = extract_client_ip(&request, Some(addr), false);

    if let Some(ref rate_limiter) = state.websocket_rate_limiter {
        match rate_limiter.check_rate_limit(client_ip) {
            RateLimitResult::Allowed(_) => {}
            RateLimitResult::Denied(rate_info) => {
                let mut headers = rate_info.to_headers();
                headers.insert("Content-Type", HeaderValue::from_static("text/plain"));

                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    headers,
                    "WebSocket upgrade rate limit exceeded",
                )
                    .into_response();
            }
        }
    }

    if !state.check_websocket_connection_limit() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Maximum WebSocket connections reached",
        )
            .into_response();
    }

    let default_world = match state.world_service.get_default_world().await {
        Ok(Some(world)) => world,
        Ok(None) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Default world not found",
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch default world: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
                .into_response();
        }
    };
    let default_world_id = default_world.id;

    ws.on_upgrade(move |socket| {
        let handler = if state.config.websocket.connection_buffer_size > 0 {
            ConnectionHandler::new_with_buffering(socket, &state, client_ip, default_world_id)
        } else {
            ConnectionHandler::new(socket, &state, client_ip, default_world_id)
        };
        handler.run(state)
    })
}

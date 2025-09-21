use serde::{Deserialize, Serialize};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

use domain::{coords::TileCoord, tile::TileVersion};
use fedi_wplace_application::contracts::subscriptions::SubscriptionRejection;

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "Information about a rejected tile subscription",
    example = json!({
        "tile": {"x": 0, "y": 0},
        "reason": "Subscription limit exceeded"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedTile {
    pub tile: TileCoord,
    #[cfg_attr(feature = "docs", schema(example = "Subscription limit exceeded"))]
    pub reason: String,
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "WebSocket messages sent from server to client. Includes subscription acknowledgments, tile version updates, error messages, and subscription confirmations. Messages use JSON format with a 'type' field to identify the message variant.",
    example = json!({
        "type": "tile-version",
        "x": 0,
        "y": 0,
        "version": "123"
    })
))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WSMessage {
    #[serde(rename = "tile-version")]
    TileVersion { x: i32, y: i32, version: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "subscription-confirmed")]
    SubscriptionConfirmed { tiles: Vec<TileCoord> },
    #[serde(rename = "subscription-ack")]
    SubscribeAck {
        accepted: Vec<TileCoord>,
        rejected: Vec<RejectedTile>,
        remaining_budget: u32,
    },
    #[serde(rename = "unsubscription-confirmed")]
    UnsubscriptionConfirmed { tiles: Vec<TileCoord> },
}

impl WSMessage {
    pub fn tile_version(coord: TileCoord, version: TileVersion) -> Self {
        Self::TileVersion {
            x: coord.x,
            y: coord.y,
            version: version.as_u64().to_string(),
        }
    }

    pub fn error(message: String) -> Self {
        Self::Error { message }
    }

    pub fn subscription_confirmed(tiles: Vec<TileCoord>) -> Self {
        Self::SubscriptionConfirmed { tiles }
    }

    pub fn subscribe_ack(
        accepted: Vec<TileCoord>,
        rejected: Vec<RejectedTile>,
        remaining_budget: u32,
    ) -> Self {
        Self::SubscribeAck {
            accepted,
            rejected,
            remaining_budget,
        }
    }

    pub fn unsubscription_confirmed(tiles: Vec<TileCoord>) -> Self {
        Self::UnsubscriptionConfirmed { tiles }
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(feature = "docs", schema(
    description = "WebSocket messages sent from client to server. Supports three message types: 'subscribe' to tiles for real-time updates, 'unsubscribe' from tiles, and 'ping' for heartbeat. Configurable maximum tile subscriptions per IP (default: 64) with FIFO eviction policy.",
    example = json!({
        "type": "subscribe",
        "tiles": [{"x": 0, "y": 0}, {"x": 1, "y": 0}]
    })
))]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { tiles: Vec<TileCoord> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { tiles: Vec<TileCoord> },
    #[serde(rename = "ping")]
    Ping,
}

impl ClientMessage {
    pub fn subscribe(tiles: Vec<TileCoord>) -> Self {
        Self::Subscribe { tiles }
    }

    pub fn unsubscribe(tiles: Vec<TileCoord>) -> Self {
        Self::Unsubscribe { tiles }
    }

    pub fn ping() -> Self {
        Self::Ping
    }
}

impl From<SubscriptionRejection> for RejectedTile {
    fn from(rejection: SubscriptionRejection) -> Self {
        Self {
            tile: rejection.tile,
            reason: rejection.reason,
        }
    }
}

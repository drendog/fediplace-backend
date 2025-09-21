use tokio::sync::broadcast::Sender;
use tracing::warn;

use domain::events::TileVersionEvent;
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::events::EventsPort,
};

pub struct TokioBroadcastEventsAdapter {
    tx: Sender<TileVersionEvent>,
}

impl TokioBroadcastEventsAdapter {
    pub fn new(tx: Sender<TileVersionEvent>) -> Self {
        Self { tx }
    }
}

impl EventsPort for TokioBroadcastEventsAdapter {
    fn broadcast_tile_version(&self, event: TileVersionEvent) -> AppResult<()> {
        self.tx.send(event).map_err(|e| {
            warn!("Failed to broadcast tile version event: {}", e);
            AppError::WebSocketError {
                message: format!("Broadcast send failed: {}", e),
            }
        })?;
        Ok(())
    }
}

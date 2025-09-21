use crate::error::AppResult;
use domain::events::TileVersionEvent;
use std::sync::Arc;

pub trait EventsPort: Send + Sync {
    fn broadcast_tile_version(&self, event: TileVersionEvent) -> AppResult<()>;
}

pub type DynEventsPort = Arc<dyn EventsPort>;

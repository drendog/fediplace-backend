use crate::error::AppResult;
use domain::{action::PaintAction, coords::TileCoord};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PixelHistoryEntry {
    pub user_id: Uuid,
    pub username: String,
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub color_id: u8,
    pub timestamp: time::OffsetDateTime,
}

#[async_trait::async_trait]
pub trait PixelHistoryStorePort: Send + Sync {
    async fn record_paint_actions(&self, actions: &[PaintAction]) -> AppResult<()>;
    async fn get_history_for_tile(&self, coord: TileCoord) -> AppResult<Vec<PixelHistoryEntry>>;
    async fn get_current_tile_state(&self, coord: TileCoord) -> AppResult<Vec<(usize, usize, u8)>>;
    async fn get_distinct_tile_count(&self, tile_size: usize) -> AppResult<i64>;
}

pub type DynPixelHistoryStorePort = Arc<dyn PixelHistoryStorePort>;

use crate::error::AppResult;
use domain::{
    action::PaintAction,
    coords::{GlobalCoord, TileCoord},
    world::WorldId,
};
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

#[derive(Debug, Clone)]
pub struct PixelInfo {
    pub user_id: Uuid,
    pub username: String,
    pub color_id: u8,
    pub timestamp: time::OffsetDateTime,
}

#[async_trait::async_trait]
pub trait PixelHistoryStorePort: Send + Sync {
    async fn record_paint_actions(
        &self,
        world_id: &WorldId,
        actions: &[PaintAction],
    ) -> AppResult<()>;
    async fn get_history_for_tile(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<Vec<PixelHistoryEntry>>;
    async fn get_current_tile_state(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<Vec<(usize, usize, u8)>>;
    async fn get_distinct_tile_count(&self, world_id: &WorldId, tile_size: usize)
    -> AppResult<i64>;
    async fn get_pixel_info(
        &self,
        world_id: &WorldId,
        coord: GlobalCoord,
    ) -> AppResult<Option<PixelInfo>>;
}

pub type DynPixelHistoryStorePort = Arc<dyn PixelHistoryStorePort>;

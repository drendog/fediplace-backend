use crate::error::AppResult;
use domain::{coords::TileCoord, world::WorldId};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait TileCachePort: Send + Sync {
    async fn get_version(&self, world_id: &WorldId, coord: TileCoord) -> AppResult<Option<u64>>;

    async fn get_palette(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
    ) -> AppResult<Option<Vec<u8>>>;
    async fn store_palette(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
        data: &[u8],
    ) -> AppResult<()>;

    async fn get_webp(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
    ) -> AppResult<Option<Vec<u8>>>;
    async fn store_webp(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
        data: &[u8],
    ) -> AppResult<()>;

    async fn has_missing_sentinel(&self, world_id: &WorldId, coord: TileCoord) -> AppResult<bool>;
    async fn set_missing_sentinel(&self, world_id: &WorldId, coord: TileCoord) -> AppResult<()>;
    async fn clear_missing_sentinel(&self, world_id: &WorldId, coord: TileCoord) -> AppResult<()>;

    async fn update_version_optimistically(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
    );
    async fn store_palette_optimistically(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
        data: &[u8],
    );

    async fn clear_cache(&self, world_id: &WorldId) -> AppResult<()>;
}

pub type DynTileCachePort = Arc<dyn TileCachePort>;

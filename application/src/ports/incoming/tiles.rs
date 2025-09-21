use crate::{
    error::AppResult,
    ports::outgoing::pixel_history_store::{PixelHistoryEntry, PixelInfo},
    tiles::{commands::PaintingResult, gateway::TileVersionResult},
};
use domain::{
    auth::UserId,
    color::ColorId,
    coords::{GlobalCoord, PixelCoord, TileCoord},
    tile::TileVersion,
};

#[async_trait::async_trait]
pub trait TilesQueryUseCase: Send + Sync {
    async fn get_tile_webp(&self, coord: TileCoord) -> AppResult<TileVersionResult>;

    async fn get_tile_version(&self, coord: TileCoord) -> AppResult<TileVersion>;
}

#[async_trait::async_trait]
pub trait PaintPixelsUseCase: Send + Sync {
    async fn paint_pixels_batch(
        &self,
        user_id: UserId,
        tile: TileCoord,
        pixels: &[(PixelCoord, ColorId)],
    ) -> AppResult<PaintingResult>;
}

#[async_trait::async_trait]
pub trait MetricsQueryUseCase: Send + Sync {
    async fn get_metrics(&self) -> AppResult<serde_json::Value>;
}

#[async_trait::async_trait]
pub trait PixelHistoryQueryUseCase: Send + Sync {
    async fn get_history_for_tile(&self, coord: TileCoord) -> AppResult<Vec<PixelHistoryEntry>>;
}

#[async_trait::async_trait]
pub trait PixelInfoQueryUseCase: Send + Sync {
    async fn get_pixel_info(&self, coord: GlobalCoord) -> AppResult<Option<PixelInfo>>;
}

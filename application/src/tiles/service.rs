use std::sync::Arc;
use tracing::{debug, instrument};

use domain::{
    action::PaintAction,
    auth::UserId,
    color::ColorId,
    coords::{PixelCoord, TileCoord},
    credits::CreditConfig,
    events::TileVersionEvent,
    tile::{PaletteBufferPool, Tile, TileVersion},
};

use crate::{
    config::TileSettings,
    error::AppResult,
    ports::{
        incoming::tiles::{
            MetricsQueryUseCase, PaintPixelsUseCase, PixelHistoryQueryUseCase, TilesQueryUseCase,
        },
        outgoing::{
            credit_store::DynCreditStorePort,
            events::DynEventsPort,
            image_codec::DynImageCodecPort,
            pixel_history_store::{DynPixelHistoryStorePort, PixelHistoryEntry},
            task_spawn::DynTaskSpawnPort,
            tile_cache::DynTileCachePort,
            timeout::DynWebPTimeoutPort,
        },
    },
};

use super::{
    commands::{PaintingResult, execute_batch_pixel_painting},
    gateway::TileGateway,
    util::validate_color_id,
};

pub type PaletteColorLookup = super::util::PaletteColorLookup;

pub struct TileServiceDeps {
    pub cache_port: DynTileCachePort,
    pub codec_port: DynImageCodecPort,
    pub webp_timeout_port: DynWebPTimeoutPort,
    pub palette_buffer_pool: Arc<PaletteBufferPool>,
    pub palette_color_lookup: Arc<PaletteColorLookup>,
    pub events_port: DynEventsPort,
    pub task_spawn_port: DynTaskSpawnPort,
    pub pixel_history_store: DynPixelHistoryStorePort,
    pub credit_store: DynCreditStorePort,
    pub credit_config: CreditConfig,
}

pub struct TileService {
    config: Arc<TileSettings>,
    repository: TileGateway,
    events_port: DynEventsPort,
    pixel_history_store: DynPixelHistoryStorePort,
    credit_store: DynCreditStorePort,
    credit_config: CreditConfig,
}

impl TileService {
    pub fn new(config: &Arc<TileSettings>, deps: TileServiceDeps) -> AppResult<Arc<Self>> {
        let repository = TileGateway::new(
            Arc::clone(config),
            deps.cache_port,
            Arc::clone(&deps.pixel_history_store),
            deps.webp_timeout_port,
            deps.palette_buffer_pool,
            deps.palette_color_lookup,
        );

        let service = Arc::new(Self {
            config: Arc::clone(config),
            repository,
            events_port: deps.events_port,
            pixel_history_store: deps.pixel_history_store,
            credit_store: deps.credit_store,
            credit_config: deps.credit_config,
        });

        Ok(service)
    }

    #[must_use]
    pub fn config(&self) -> &TileSettings {
        &self.config
    }

    #[must_use]
    pub fn repository(&self) -> &TileGateway {
        &self.repository
    }

    #[instrument(skip(self))]
    pub async fn get_tile_webp(
        &self,
        coord: TileCoord,
    ) -> AppResult<super::gateway::TileVersionResult> {
        coord.validate_bounds()?;
        self.repository.get_tile_webp(coord).await
    }

    async fn load_tile_for_painting(&self, tile_coord: TileCoord) -> AppResult<Arc<Tile>> {
        tile_coord.validate_bounds()?;
        self.repository.load_tile_for_painting(tile_coord).await
    }

    async fn store_palette_in_redis(
        &self,
        tile_coord: TileCoord,
        version: u64,
        tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        self.repository
            .store_palette_in_cache(tile_coord, version, tile_arc)
            .await
    }

    async fn update_cache_optimistically(
        &self,
        tile_coord: TileCoord,
        version: u64,
        tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        self.repository
            .update_cache_optimistically(tile_coord, version, tile_arc)
            .await
    }

    #[instrument(skip(self, pixels))]
    pub async fn paint_pixels_batch(
        &self,
        user_id: UserId,
        tile_coord: TileCoord,
        pixels: &[(PixelCoord, ColorId)],
    ) -> AppResult<PaintingResult> {
        tile_coord.validate_bounds()?;

        for (pixel_coord, color_id) in pixels {
            pixel_coord.validate_bounds(self.config.tile_size)?;
            validate_color_id(color_id.id(), &self.config.color_palette_config)?;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let pixel_count = pixels.len() as i32;
        self.credit_store
            .spend_user_credits(&user_id, pixel_count, &self.credit_config)
            .await?;

        debug!("Painting {} pixels on tile {}", pixels.len(), tile_coord);

        let tile_arc = self.load_tile_for_painting(tile_coord).await?;
        let painting_result =
            execute_batch_pixel_painting(tile_coord, pixels, &tile_arc, &self.config)?;

        debug!(
            "Paint complete, tile {} now v{}",
            tile_coord, painting_result.new_version
        );

        self.store_palette_in_redis(tile_coord, painting_result.new_version, &tile_arc)
            .await?;

        self.update_cache_optimistically(tile_coord, painting_result.new_version, &tile_arc)
            .await?;

        let paint_actions: Vec<PaintAction> = pixels
            .iter()
            .map(|(pixel_coord, color_id)| {
                PaintAction::from_tile_and_pixel(
                    user_id.clone(),
                    tile_coord,
                    *pixel_coord,
                    *color_id,
                    time::OffsetDateTime::now_utc(),
                    self.config.tile_size,
                )
            })
            .collect();

        self.pixel_history_store
            .record_paint_actions(&paint_actions)
            .await?;

        self.events_port
            .broadcast_tile_version(TileVersionEvent {
                coord: tile_coord,
                version: painting_result.new_version,
            })
            .ok();

        debug!(
            "Paint complete - paint v{}, WS v{}",
            painting_result.new_version, painting_result.new_version
        );

        Ok(painting_result)
    }

    #[instrument(skip(self))]
    pub async fn get_tile_version(&self, coord: TileCoord) -> AppResult<TileVersion> {
        self.repository.get_tile_version(coord).await
    }

    pub async fn get_metrics(&self) -> AppResult<serde_json::Value> {
        let tile_count = self.repository.get_distinct_tile_count().await?;

        Ok(serde_json::json!({
            "total_tiles": tile_count,
            "pipeline": "pixel_history_only"
        }))
    }
}

#[async_trait::async_trait]
impl TilesQueryUseCase for TileService {
    async fn get_tile_webp(
        &self,
        coord: TileCoord,
    ) -> AppResult<super::gateway::TileVersionResult> {
        self.get_tile_webp(coord).await
    }

    async fn get_tile_version(&self, coord: TileCoord) -> AppResult<TileVersion> {
        self.get_tile_version(coord).await
    }
}

#[async_trait::async_trait]
impl PaintPixelsUseCase for TileService {
    async fn paint_pixels_batch(
        &self,
        user_id: UserId,
        tile: TileCoord,
        pixels: &[(PixelCoord, ColorId)],
    ) -> AppResult<PaintingResult> {
        self.paint_pixels_batch(user_id, tile, pixels).await
    }
}

#[async_trait::async_trait]
impl MetricsQueryUseCase for TileService {
    async fn get_metrics(&self) -> AppResult<serde_json::Value> {
        self.get_metrics().await
    }
}

#[async_trait::async_trait]
impl PixelHistoryQueryUseCase for TileService {
    async fn get_history_for_tile(&self, coord: TileCoord) -> AppResult<Vec<PixelHistoryEntry>> {
        coord.validate_bounds()?;
        self.pixel_history_store.get_history_for_tile(coord).await
    }
}

use super::{
    commands::{PaintingResult, execute_batch_pixel_painting},
    gateway::TileGateway,
    util::validate_color_id,
};
use crate::{
    config::TileSettings,
    error::{AppError, AppResult},
    ports::{
        incoming::tiles::{
            MetricsQueryUseCase, PaintPixelsUseCase, PixelHistoryQueryUseCase,
            PixelInfoQueryUseCase, TilesQueryUseCase,
        },
        outgoing::{
            credit_store::DynCreditStorePort,
            events::DynEventsPort,
            image_codec::DynImageCodecPort,
            pixel_history_store::{DynPixelHistoryStorePort, PixelHistoryEntry, PixelInfo},
            task_spawn::DynTaskSpawnPort,
            tile_cache::DynTileCachePort,
            timeout::DynWebPTimeoutPort,
        },
    },
};
use domain::{
    action::PaintAction,
    auth::UserId,
    color::ColorId,
    coords::{GlobalCoord, PixelCoord, TileCoord},
    credits::CreditBalance,
    events::TileVersionEvent,
    tile::{PaletteBufferPool, Tile, TileVersion},
    world::WorldId,
};
use std::sync::Arc;
use tracing::{debug, instrument};

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
}

pub struct TileService {
    config: Arc<TileSettings>,
    repository: TileGateway,
    events_port: DynEventsPort,
    pixel_history_store: DynPixelHistoryStorePort,
    credit_store: DynCreditStorePort,
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
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<super::gateway::TileVersionResult> {
        coord.validate_bounds()?;
        self.repository.get_tile_webp(world_id, coord).await
    }

    async fn load_tile_for_painting(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
    ) -> AppResult<Arc<Tile>> {
        tile_coord.validate_bounds()?;
        self.repository
            .load_tile_for_painting(world_id, tile_coord)
            .await
    }

    async fn store_palette_in_redis(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
        version: u64,
        tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        self.repository
            .store_palette_in_cache(world_id, tile_coord, version, tile_arc)
            .await
    }

    async fn update_cache_optimistically(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
        version: u64,
        tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        self.repository
            .update_cache_optimistically(world_id, tile_coord, version, tile_arc)
            .await
    }

    #[instrument(skip(self, pixels))]
    pub async fn paint_pixels_batch(
        &self,
        world_id: &WorldId,
        user_id: UserId,
        tile_coord: TileCoord,
        pixels: &[(PixelCoord, ColorId)],
    ) -> AppResult<PaintingResult> {
        tile_coord.validate_bounds()?;

        for (pixel_coord, color_id) in pixels {
            pixel_coord.validate_bounds(self.config.tile_size)?;
            validate_color_id(color_id.id(), &self.config.palette)?;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let pixel_count = pixels.len() as i32;

        let balance = self.credit_store.get_user_credits(&user_id).await?;

        if balance.available_charges < pixel_count {
            return Err(AppError::InsufficientCredits {
                message: format!(
                    "Required {} charges, but only {} available",
                    pixel_count, balance.available_charges
                ),
            });
        }

        let new_balance = CreditBalance {
            available_charges: balance.available_charges - pixel_count,
            charges_updated_at: time::OffsetDateTime::now_utc(),
        };

        self.credit_store
            .update_user_credits(&user_id, &new_balance)
            .await?;

        debug!(
            "Painting {} pixels on tile {} in world {:?}",
            pixels.len(),
            tile_coord,
            world_id
        );

        let tile_arc = self.load_tile_for_painting(world_id, tile_coord).await?;
        let painting_result =
            execute_batch_pixel_painting(tile_coord, pixels, &tile_arc, &self.config)?;

        debug!(
            "Paint complete, tile {} now v{}",
            tile_coord, painting_result.new_version
        );

        self.store_palette_in_redis(world_id, tile_coord, painting_result.new_version, &tile_arc)
            .await?;

        self.update_cache_optimistically(
            world_id,
            tile_coord,
            painting_result.new_version,
            &tile_arc,
        )
        .await?;

        let paint_actions: Vec<PaintAction> = pixels
            .iter()
            .map(|(pixel_coord, color_id)| {
                PaintAction::from_tile_and_pixel(
                    world_id.clone(),
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
            .record_paint_actions(world_id, &paint_actions)
            .await?;

        self.events_port
            .broadcast_tile_version(TileVersionEvent {
                world_id: world_id.clone(),
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
    pub async fn get_tile_version(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<TileVersion> {
        self.repository.get_tile_version(world_id, coord).await
    }

    pub async fn get_metrics(&self, world_id: &WorldId) -> AppResult<serde_json::Value> {
        let tile_count = self.repository.get_distinct_tile_count(world_id).await?;

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
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<super::gateway::TileVersionResult> {
        self.get_tile_webp(world_id, coord).await
    }

    async fn get_tile_version(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<TileVersion> {
        self.get_tile_version(world_id, coord).await
    }
}

#[async_trait::async_trait]
impl PaintPixelsUseCase for TileService {
    async fn paint_pixels_batch(
        &self,
        world_id: &WorldId,
        user_id: UserId,
        tile: TileCoord,
        pixels: &[(PixelCoord, ColorId)],
    ) -> AppResult<PaintingResult> {
        self.paint_pixels_batch(world_id, user_id, tile, pixels)
            .await
    }
}

#[async_trait::async_trait]
impl MetricsQueryUseCase for TileService {
    async fn get_metrics(&self, world_id: &WorldId) -> AppResult<serde_json::Value> {
        self.get_metrics(world_id).await
    }
}

#[async_trait::async_trait]
impl PixelHistoryQueryUseCase for TileService {
    async fn get_history_for_tile(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<Vec<PixelHistoryEntry>> {
        coord.validate_bounds()?;
        self.pixel_history_store
            .get_history_for_tile(world_id, coord)
            .await
    }
}

#[async_trait::async_trait]
impl PixelInfoQueryUseCase for TileService {
    async fn get_pixel_info(
        &self,
        world_id: &WorldId,
        coord: GlobalCoord,
    ) -> AppResult<Option<PixelInfo>> {
        coord.validate()?;
        self.pixel_history_store
            .get_pixel_info(world_id, coord)
            .await
    }
}

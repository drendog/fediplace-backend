use crate::{
    config::TileSettings,
    error::{AppError, AppResult},
    ports::outgoing::{
        pixel_history_store::DynPixelHistoryStorePort, tile_cache::DynTileCachePort,
        timeout::DynWebPTimeoutPort,
    },
};
use domain::{
    coords::TileCoord,
    tile::{PaletteBufferPool, Tile, TileVersion},
    world::WorldId,
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tracing::debug;

use super::util::{PaletteColorLookup, palette_to_rgba_pixels, populate_tile_from_rgba};

pub struct TileVersionResult {
    pub webp_data: Vec<u8>,
    pub version: TileVersion,
    pub etag: Option<String>,
}

pub struct CacheHierarchyResult {
    pub rgba_pixels: Vec<u32>,
    pub cache_level: CacheLevel,
}

pub enum CacheLevel {
    Redis,
    Database,
    CompleteMissWithSentinel,
}

#[derive(Debug)]
pub enum VersionSource {
    Redis,
    Database,
    EmptyTile,
}

pub struct VersionLookupResult {
    pub version: u64,
    pub source: VersionSource,
}

#[derive(Clone)]
pub struct TileGateway {
    config: Arc<TileSettings>,
    cache_port: DynTileCachePort,
    pixel_history_store: DynPixelHistoryStorePort,
    webp_timeout_port: DynWebPTimeoutPort,
    palette_buffer_pool: Arc<PaletteBufferPool>,
    palette_color_lookup: Arc<PaletteColorLookup>,
}

impl TileGateway {
    pub fn new(
        config: Arc<TileSettings>,
        cache_port: DynTileCachePort,
        pixel_history_store: DynPixelHistoryStorePort,
        webp_timeout_port: DynWebPTimeoutPort,
        palette_buffer_pool: Arc<PaletteBufferPool>,
        palette_color_lookup: Arc<PaletteColorLookup>,
    ) -> Self {
        Self {
            config,
            cache_port,
            pixel_history_store,
            webp_timeout_port,
            palette_buffer_pool,
            palette_color_lookup,
        }
    }

    pub async fn get_distinct_tile_count(&self, world_id: &WorldId) -> AppResult<i64> {
        self.pixel_history_store
            .get_distinct_tile_count(world_id, self.config.tile_size)
            .await
    }

    pub fn palette_buffer_pool(&self) -> &Arc<PaletteBufferPool> {
        &self.palette_buffer_pool
    }

    pub async fn get_tile_webp(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<TileVersionResult> {
        debug!("Getting WebP for tile {} in world {:?}", coord, world_id);

        let version_lookup = self
            .find_authoritative_tile_version(world_id, coord)
            .await?;

        debug!(
            "Found authoritative version {} for tile {} from {:?}",
            version_lookup.version, coord, version_lookup.source
        );

        let etag = Some(format!("\"{}\"", version_lookup.version));

        if let Some(cached_webp) = self
            .try_get_cached_webp(world_id, coord, version_lookup.version, etag.clone())
            .await?
        {
            return Ok(cached_webp);
        }

        self.generate_and_cache_webp(world_id, coord, version_lookup.version, etag)
            .await
    }

    pub async fn get_tile_rgba(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<CacheHierarchyResult> {
        let authoritative_version_lookup = self
            .find_authoritative_tile_version(world_id, coord)
            .await?;

        self.load_rgba_pixels_from_cache_or_database(
            world_id,
            coord,
            authoritative_version_lookup.version,
        )
        .await
    }

    pub async fn load_tile_for_painting(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
    ) -> AppResult<Arc<Tile>> {
        let authoritative_version_lookup = self
            .find_authoritative_tile_version(world_id, tile_coord)
            .await?;

        if let Some(palette_bytes) = self
            .cache_port
            .get_palette(world_id, tile_coord, authoritative_version_lookup.version)
            .await?
        {
            let rgba_pixels = palette_to_rgba_pixels(&palette_bytes, &self.config.palette);
            let tile = Arc::new(Tile::new(tile_coord, self.config.tile_size));
            populate_tile_from_rgba(&tile, &rgba_pixels, &self.palette_color_lookup)?;
            tile.mark_clean(authoritative_version_lookup.version);
            return Ok(tile);
        }

        let pixel_state = self
            .pixel_history_store
            .get_current_tile_state(world_id, tile_coord)
            .await?;
        let tile = if pixel_state.is_empty() {
            Arc::new(Tile::new(tile_coord, self.config.tile_size))
        } else {
            self.reconstruct_tile_from_pixel_history(tile_coord, &pixel_state)
        };

        tile.mark_clean(authoritative_version_lookup.version);
        Ok(tile)
    }

    pub async fn store_palette_in_cache(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
        version: u64,
        tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        let (_, palette_data) = tile_arc.snapshot_palette(&self.palette_buffer_pool);

        self.cache_port
            .store_palette(world_id, tile_coord, version, &palette_data)
            .await?;

        self.palette_buffer_pool.release_buffer(palette_data);
        Ok(())
    }

    pub async fn update_cache_optimistically(
        &self,
        world_id: &WorldId,
        tile_coord: TileCoord,
        version: u64,
        _tile_arc: &Arc<Tile>,
    ) -> AppResult<()> {
        self.cache_port
            .update_version_optimistically(world_id, tile_coord, version)
            .await;
        Ok(())
    }

    async fn load_rgba_pixels_from_cache_or_database(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        authoritative_version: u64,
    ) -> AppResult<CacheHierarchyResult> {
        if let Some(palette_bytes) = self
            .cache_port
            .get_palette(world_id, coord, authoritative_version)
            .await?
        {
            let rgba_pixels = palette_to_rgba_pixels(&palette_bytes, &self.config.palette);
            return Ok(CacheHierarchyResult {
                rgba_pixels,
                cache_level: CacheLevel::Redis,
            });
        }

        self.load_rgba_pixels_from_database_and_populate_caches(world_id, coord)
            .await
    }

    async fn load_rgba_pixels_from_database_and_populate_caches(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<CacheHierarchyResult> {
        let pixel_state = self
            .pixel_history_store
            .get_current_tile_state(world_id, coord)
            .await?;

        if pixel_state.is_empty() {
            return self.handle_complete_cache_miss(world_id, coord).await;
        }

        let tile = self.reconstruct_tile_from_pixel_history(coord, &pixel_state);
        let (_, palette_data) = tile.snapshot_palette(&self.palette_buffer_pool);
        let rgba_pixels = palette_to_rgba_pixels(&palette_data, &self.config.palette);

        let version = pixel_state.len() as u64;

        self.cache_port
            .store_palette_optimistically(world_id, coord, version, &palette_data)
            .await;
        self.cache_port
            .update_version_optimistically(world_id, coord, version)
            .await;

        self.palette_buffer_pool.release_buffer(palette_data);

        debug!(
            "Loaded tile {} v{} from pixel history with {} pixels",
            coord,
            version,
            pixel_state.len()
        );

        Ok(CacheHierarchyResult {
            rgba_pixels,
            cache_level: CacheLevel::Database,
        })
    }

    fn reconstruct_tile_from_pixel_history(
        &self,
        coord: TileCoord,
        pixel_state: &[(usize, usize, i16)],
    ) -> Arc<Tile> {
        let tile = Arc::new(Tile::new(coord, self.config.tile_size));

        for &(x, y, color_id) in pixel_state {
            if x < self.config.tile_size && y < self.config.tile_size {
                let index = y * self.config.tile_size + x;
                if let Some(pixel) = tile.pixels.get(index) {
                    pixel.store(color_id, Ordering::Relaxed);
                }
            }
        }

        tile
    }

    async fn handle_complete_cache_miss(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<CacheHierarchyResult> {
        if self
            .cache_port
            .has_missing_sentinel(world_id, coord)
            .await?
        {
            debug!(
                "Found missing sentinel for tile {}, returning empty buffer",
                coord
            );
            return Ok(self.create_empty_tile_data(coord));
        }

        debug!("Complete cache miss for tile {}, setting sentinel", coord);

        self.cache_port
            .set_missing_sentinel(world_id, coord)
            .await
            .ok();
        Ok(self.create_empty_tile_data(coord))
    }

    fn create_empty_tile_data(&self, coord: TileCoord) -> CacheHierarchyResult {
        debug!("Creating empty tile for {}", coord);
        let new_tile = Arc::new(Tile::new(coord, self.config.tile_size));

        CacheHierarchyResult {
            rgba_pixels: {
                let (_, palette_data) = new_tile.snapshot_palette(&self.palette_buffer_pool);
                let rgba_pixels = palette_to_rgba_pixels(&palette_data, &self.config.palette);
                self.palette_buffer_pool.release_buffer(palette_data);
                rgba_pixels
            },
            cache_level: CacheLevel::CompleteMissWithSentinel,
        }
    }

    async fn encode_webp_from_rgba(&self, rgba_pixels: Vec<u32>) -> AppResult<Vec<u8>> {
        self.webp_timeout_port
            .encode_webp_with_timeout(rgba_pixels, Duration::from_secs(3))
            .await
            .map_err(|_| AppError::CodecError {
                message: "WebP encoding timeout".to_string(),
            })
    }

    pub async fn get_tile_version(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<TileVersion> {
        let authoritative_version_lookup = self
            .find_authoritative_tile_version(world_id, coord)
            .await?;
        Ok(TileVersion::from_u64(authoritative_version_lookup.version))
    }

    async fn find_authoritative_tile_version(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<VersionLookupResult> {
        if let Some(version) = self.cache_port.get_version(world_id, coord).await? {
            return Ok(VersionLookupResult {
                version,
                source: VersionSource::Redis,
            });
        }

        if self
            .cache_port
            .has_missing_sentinel(world_id, coord)
            .await?
        {
            debug!("Found missing sentinel for tile {}", coord);
            return Ok(VersionLookupResult {
                version: 0,
                source: VersionSource::EmptyTile,
            });
        }

        let pixel_state = self
            .pixel_history_store
            .get_current_tile_state(world_id, coord)
            .await?;

        if pixel_state.is_empty() {
            debug!("No pixels found for tile {}, using v0", coord);
            Ok(VersionLookupResult {
                version: 0,
                source: VersionSource::EmptyTile,
            })
        } else {
            let version = pixel_state.len() as u64;
            debug!(
                "Found version {} from pixel history for tile {}",
                version, coord
            );
            Ok(VersionLookupResult {
                version,
                source: VersionSource::Database,
            })
        }
    }

    async fn try_get_cached_webp(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
        etag: Option<String>,
    ) -> AppResult<Option<TileVersionResult>> {
        if let Some(webp_data) = self.cache_port.get_webp(world_id, coord, version).await? {
            debug!(
                "WebP cache hit for tile {} v{} ({} bytes)",
                coord,
                version,
                webp_data.len()
            );
            return Ok(Some(TileVersionResult {
                webp_data,
                version: TileVersion::from_u64(version),
                etag,
            }));
        }
        Ok(None)
    }

    async fn generate_and_cache_webp(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
        etag: Option<String>,
    ) -> AppResult<TileVersionResult> {
        debug!("Generating WebP for tile {} v{}", coord, version);

        let rgba_hierarchy_result = self
            .load_rgba_pixels_from_cache_or_database(world_id, coord, version)
            .await?;
        let webp_data = self
            .encode_webp_from_rgba(rgba_hierarchy_result.rgba_pixels)
            .await?;

        self.cache_port
            .store_webp(world_id, coord, version, &webp_data)
            .await?;

        debug!(
            "Generated WebP for tile {} v{} ({} bytes)",
            coord,
            version,
            webp_data.len()
        );
        Ok(TileVersionResult {
            webp_data,
            version: TileVersion::from_u64(version),
            etag,
        })
    }

    pub async fn get_palette_from_cache(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
    ) -> AppResult<Option<Vec<i16>>> {
        self.cache_port.get_palette(world_id, coord, version).await
    }

    pub async fn clear_missing_sentinel(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<()> {
        self.cache_port
            .clear_missing_sentinel(world_id, coord)
            .await
    }

    pub async fn update_version_optimistically(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
        version: u64,
    ) {
        self.cache_port
            .update_version_optimistically(world_id, coord, version)
            .await;
    }
}

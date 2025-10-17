use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU64, Ordering};
#[cfg(feature = "docs")]
use utoipa::ToSchema;

use crate::color::ColorId;
use crate::coords::{PixelCoord, TileCoord};
use crate::error::{DomainError, DomainResult};

pub struct PaletteBufferPool {
    buffers: Arc<ArrayQueue<Vec<i16>>>,
    buffer_size_pixels: usize,
}

impl PaletteBufferPool {
    #[must_use]
    pub fn new(tile_size: usize, max_pooled_buffers: usize) -> Self {
        let buffer_size_pixels = tile_size * tile_size;
        Self {
            buffers: Arc::new(ArrayQueue::new(max_pooled_buffers)),
            buffer_size_pixels,
        }
    }

    #[must_use]
    pub fn acquire_buffer(&self) -> Vec<i16> {
        if let Some(mut buffer) = self.buffers.pop() {
            buffer.clear();
            buffer.resize(self.buffer_size_pixels, ColorId::TRANSPARENT);
            return buffer;
        }
        vec![ColorId::TRANSPARENT; self.buffer_size_pixels]
    }

    pub fn release_buffer(&self, buffer: Vec<i16>) {
        self.buffers.push(buffer).ok();
    }
}

pub struct Tile {
    pub pixels: Box<[AtomicI16]>,
    pub dirty: AtomicBool,
    pub version: AtomicU64,
    pub dirty_since: AtomicU64,
    pub coord: TileCoord,
    pub tile_size: usize,
}

impl Tile {
    #[must_use]
    pub fn new(coord: TileCoord, tile_size: usize) -> Self {
        let total_pixels = tile_size * tile_size;
        let mut pixels = Vec::with_capacity(total_pixels);
        for _ in 0..total_pixels {
            pixels.push(AtomicI16::new(ColorId::TRANSPARENT));
        }

        Self {
            pixels: pixels.into_boxed_slice(),
            dirty: AtomicBool::new(false),
            version: AtomicU64::new(1),
            dirty_since: AtomicU64::new(u64::MAX),
            coord,
            tile_size,
        }
    }

    fn total_pixels(&self) -> usize {
        self.tile_size * self.tile_size
    }

    pub fn paint_pixels_batch(
        &self,
        pixels: &[(PixelCoord, ColorId)],
        pixel_size: usize,
    ) -> DomainResult<u64> {
        if pixels.is_empty() {
            return Err(DomainError::InvalidPixelCoordinates(
                "Cannot paint empty pixel batch".to_string(),
            ));
        }

        for (pixel_coord, color_id) in pixels {
            let snapped_coord = pixel_coord.snap_to_grid(pixel_size);
            let palette_id = color_id.id();

            let start_x = snapped_coord.x;
            let start_y = snapped_coord.y;
            let end_x = (start_x + pixel_size).min(self.tile_size);
            let end_y = (start_y + pixel_size).min(self.tile_size);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    if x < self.tile_size && y < self.tile_size {
                        let index = y * self.tile_size + x;
                        if let Some(pixel) = self.pixels.get(index) {
                            pixel.store(palette_id, Ordering::Release);
                        }
                    }
                }
            }
        }

        self.dirty.store(true, Ordering::Relaxed);
        let new_version = self.increment_version();
        self.update_dirty_since(new_version);

        Ok(new_version)
    }

    pub fn mark_clean(&self, persisted_version: u64) {
        self.dirty.store(false, Ordering::Relaxed);
        self.version.store(persisted_version, Ordering::Relaxed);
        self.dirty_since.store(u64::MAX, Ordering::Relaxed);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    pub fn get_version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    pub fn increment_version(&self) -> u64 {
        self.version.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn update_dirty_since(&self, version_when_dirtied: u64) {
        let mut current_earliest = self.dirty_since.load(Ordering::Relaxed);
        while current_earliest > version_when_dirtied {
            match self.dirty_since.compare_exchange_weak(
                current_earliest,
                version_when_dirtied,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed_value) => current_earliest = observed_value,
            }
        }
    }

    pub fn snapshot_palette(&self, palette_pool: &PaletteBufferPool) -> (u64, Vec<i16>) {
        loop {
            let version_before = self.version.load(Ordering::Acquire);

            let mut buffer = palette_pool.acquire_buffer();
            let total_pixels = self.total_pixels();

            for (i, pixel) in self.pixels.iter().enumerate().take(total_pixels) {
                let value = pixel.load(Ordering::Acquire);
                if let Some(slot) = buffer.get_mut(i) {
                    *slot = value;
                }
            }

            let version_after = self.version.load(Ordering::Acquire);

            if version_before == version_after {
                return (version_before, buffer);
            }

            palette_pool.release_buffer(buffer);
        }
    }

    pub fn populate_from_palette(&self, palette_data: &[i16]) -> DomainResult<()> {
        let expected_pixels = self.total_pixels();
        if palette_data.len() != expected_pixels {
            return Err(DomainError::CodecError(format!(
                "Expected {expected_pixels} pixels, got {}",
                palette_data.len()
            )));
        }

        for (pixel, &palette_id) in self.pixels.iter().zip(palette_data.iter()) {
            pixel.store(palette_id, Ordering::Relaxed);
        }

        Ok(())
    }
}

#[cfg_attr(feature = "docs", derive(ToSchema))]
#[cfg_attr(
    feature = "docs",
    schema(
        description = "Tile version number for cache invalidation and consistency",
        example = 1_234_567_890
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileVersion(u64);

impl TileVersion {
    #[must_use]
    pub fn new() -> Self {
        Self(1)
    }

    #[must_use]
    pub fn from_u64(version: u64) -> Self {
        Self(version)
    }

    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl Default for TileVersion {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TileVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

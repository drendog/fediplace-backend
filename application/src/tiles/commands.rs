use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::debug;

use crate::{config::TileSettings, error::AppResult};
use domain::{
    color::ColorId,
    coords::{PixelCoord, TileCoord},
    tile::Tile,
};

pub struct PaintingResult {
    pub new_version: u64,
    pub write_id: String,
}

pub fn execute_batch_pixel_painting(
    tile_coord: TileCoord,
    pixels: &[(PixelCoord, ColorId)],
    tile: &Arc<Tile>,
    config: &TileSettings,
) -> AppResult<PaintingResult> {
    let pixel_size = config.pixel_size;

    let new_version = tile.paint_pixels_batch(pixels, pixel_size)?;

    let write_id = format!(
        "{:x}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        new_version
    );

    debug!(
        "Painted {} pixels at tile {} -> v{} writeId={}",
        pixels.len(),
        tile_coord,
        new_version,
        write_id
    );

    Ok(PaintingResult {
        new_version,
        write_id,
    })
}

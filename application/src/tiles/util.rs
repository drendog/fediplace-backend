use std::{collections::HashMap, sync::atomic::Ordering};

use crate::error::{AppError, AppResult};
use domain::color::ColorId;
use domain::{
    color::{Color, RgbColor},
    tile::Tile,
};

const TRANSPARENT_COLOR: u32 = 0;

#[derive(Debug, Clone)]
pub struct PaletteColorLookup {
    color_to_id: HashMap<RgbColor, u8>,
}

impl PaletteColorLookup {
    pub fn from_color_palette(colors: &[RgbColor]) -> Self {
        let mut color_to_id = HashMap::with_capacity(colors.len());

        for (index, color) in colors.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let id = index as u8;
            color_to_id.insert(*color, id);
        }

        Self { color_to_id }
    }

    pub fn find_color_id(&self, color: RgbColor) -> Option<u8> {
        self.color_to_id.get(&color).copied()
    }

    pub fn color_count(&self) -> usize {
        self.color_to_id.len()
    }
}

pub fn palette_to_rgba_pixels(palette_ids: &[i16], color_palette: &[RgbColor]) -> Vec<u32> {
    let mut rgba_pixels = Vec::with_capacity(palette_ids.len());
    for &palette_id in palette_ids {
        let color_id = ColorId::new(palette_id);

        if color_id.is_transparent() {
            rgba_pixels.push(TRANSPARENT_COLOR);
        } else if let Some(color) = color_palette.get(color_id.id() as usize) {
            rgba_pixels.push(color.to_rgba_u32());
        } else {
            rgba_pixels.push(TRANSPARENT_COLOR);
        }
    }
    rgba_pixels
}

pub fn populate_tile_from_rgba(
    tile: &Tile,
    rgba_pixel_data: &[u32],
    palette_lookup: &PaletteColorLookup,
) -> AppResult<()> {
    let expected_pixels = tile.tile_size * tile.tile_size;
    if rgba_pixel_data.len() != expected_pixels {
        return Err(AppError::CodecError {
            message: format!(
                "Expected {} pixels, got {}",
                expected_pixels,
                rgba_pixel_data.len()
            ),
        });
    }

    for (pixel, &rgba_value) in tile.pixels.iter().zip(rgba_pixel_data.iter()) {
        let palette_id = if rgba_value == TRANSPARENT_COLOR {
            ColorId::TRANSPARENT
        } else {
            let color = Color::from_rgba_u32(rgba_value);
            i16::from(palette_lookup.find_color_id(color).ok_or_else(|| {
                AppError::InvalidColorFormat {
                    message: format!(
                        "Color not found in palette: ({}, {}, {})",
                        color.r, color.g, color.b
                    ),
                }
            })?)
        };
        pixel.store(palette_id, Ordering::Relaxed);
    }

    Ok(())
}

pub fn validate_color_id(id: i16, palette: &[RgbColor]) -> AppResult<()> {
    use domain::color::ColorId;

    if id == ColorId::TRANSPARENT {
        return Ok(());
    }

    if id < 0 {
        return Err(AppError::InvalidColorFormat {
            message: format!("Color ID {} is invalid (must be -1 or 0-255)", id),
        });
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let color_count = palette.len() as i16;

    if id >= color_count {
        return Err(AppError::InvalidColorFormat {
            message: format!(
                "Color ID {} is out of bounds (valid range: -1 (transparent) or 0-{})",
                id,
                color_count.saturating_sub(1)
            ),
        });
    }

    Ok(())
}

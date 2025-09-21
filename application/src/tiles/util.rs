use std::{collections::HashMap, sync::atomic::Ordering};

use crate::error::{AppError, AppResult};
use crate::infrastructure_config::ColorPaletteConfig;
use domain::{
    color::{Color, RgbColor},
    tile::Tile,
};

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

pub fn palette_to_rgba_pixels(palette_bytes: &[u8], color_palette: &[RgbColor]) -> Vec<u32> {
    let mut rgba_pixels = Vec::with_capacity(palette_bytes.len());
    for &palette_id in palette_bytes {
        if let Some(color) = color_palette.get(palette_id as usize) {
            rgba_pixels.push(color.to_rgba_u32());
        } else {
            rgba_pixels.push(Color::transparent_rgba_u32());
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
        if rgba_value == Color::transparent_rgba_u32() {
            pixel.store(tile.transparency_color_id(), Ordering::Relaxed);
        } else {
            let color = Color::from_rgba_u32(rgba_value);
            let palette_id = palette_lookup.find_color_id(color).ok_or_else(|| {
                AppError::InvalidColorFormat {
                    message: format!(
                        "Color not found in palette: ({}, {}, {})",
                        color.r, color.g, color.b
                    ),
                }
            })?;
            pixel.store(palette_id, Ordering::Relaxed);
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColorRules {
    max_regular_id: u8,
    transparency_id: Option<u8>,
    total_valid_ids: u8,
}

#[allow(dead_code)]
impl ColorRules {
    pub fn new(palette_cfg: &ColorPaletteConfig) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let regular_color_count = palette_cfg.colors.len() as u8;
        let max_regular_id = regular_color_count.saturating_sub(1);
        let transparency_id = palette_cfg.get_transparency_color_id();

        #[allow(clippy::cast_possible_truncation)]
        let total_valid_ids = regular_color_count + palette_cfg.special_colors.len() as u8;

        Self {
            max_regular_id,
            transparency_id,
            total_valid_ids,
        }
    }

    pub fn is_valid_for_painting(&self, color_id: u8) -> bool {
        color_id < self.total_valid_ids
            && (color_id <= self.max_regular_id || Some(color_id) == self.transparency_id)
    }

    pub fn is_in_bounds(&self, color_id: u8) -> bool {
        color_id < self.total_valid_ids
    }
}

pub fn validate_color_id(id: u8, palette_cfg: &ColorPaletteConfig) -> AppResult<()> {
    #[allow(clippy::cast_possible_truncation)]
    let regular_color_count = palette_cfg.colors.len() as u8;
    #[allow(clippy::cast_possible_truncation)]
    let total_colors = regular_color_count + palette_cfg.special_colors.len() as u8;

    if id >= total_colors {
        return Err(AppError::InvalidColorFormat {
            message: format!(
                "Color ID {} is out of bounds (valid range: 0-{})",
                id,
                total_colors.saturating_sub(1)
            ),
        });
    }

    if id >= regular_color_count {
        let special_index = (id - regular_color_count) as usize;
        if let Some(purpose) = palette_cfg.special_colors.get(special_index) {
            if purpose != "transparency" {
                return Err(AppError::InvalidColorFormat {
                    message: format!(
                        "Special color '{purpose}' (ID {id}) cannot be used for painting"
                    ),
                });
            }
        }
    }

    Ok(())
}

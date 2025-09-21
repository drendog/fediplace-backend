use crate::infrastructure_config::ColorPaletteConfig;
use domain::color::RgbColor;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TileSettings {
    pub tile_size: usize,
    pub pixel_size: usize,
    pub palette: Arc<[RgbColor]>,
    pub transparency_color_id: u8,
    pub color_palette_config: Arc<ColorPaletteConfig>,
}

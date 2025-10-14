use crate::color::RgbColor;
use crate::world::WorldId;

#[derive(Debug, Clone)]
pub struct CanvasConfig {
    pub default_world_id: WorldId,
    pub palette: Vec<RgbColor>,
    pub special_colors: Vec<(u8, String)>,
}

impl CanvasConfig {
    pub fn new(
        default_world_id: WorldId,
        palette: Vec<RgbColor>,
        special_colors: Vec<(u8, String)>,
    ) -> Self {
        Self {
            default_world_id,
            palette,
            special_colors,
        }
    }
}

use crate::color::RgbColor;
use crate::world::WorldId;

#[derive(Debug, Clone)]
pub struct CanvasConfig {
    pub default_world_id: WorldId,
    pub palette: Vec<RgbColor>,
}

impl CanvasConfig {
    pub fn new(default_world_id: WorldId, palette: Vec<RgbColor>) -> Self {
        Self {
            default_world_id,
            palette,
        }
    }
}

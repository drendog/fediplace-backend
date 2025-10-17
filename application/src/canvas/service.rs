use crate::error::{AppError, AppResult};
use crate::ports::outgoing::world_store::DynWorldStorePort;
use domain::canvas_config::CanvasConfig;
use domain::color::RgbColor;

pub struct CanvasConfigService {
    world_store: DynWorldStorePort,
}

impl CanvasConfigService {
    pub fn new(world_store: DynWorldStorePort) -> Self {
        Self { world_store }
    }

    pub async fn get_canvas_config(&self) -> AppResult<CanvasConfig> {
        let default_world =
            self.world_store
                .get_default_world()
                .await?
                .ok_or_else(|| AppError::NotFound {
                    message: "No default world found".to_string(),
                })?;

        let palette_colors = self
            .world_store
            .get_palette_colors(&default_world.id)
            .await?;

        let mut palette = Vec::new();

        for palette_color in palette_colors {
            if let Some(rgba_u32) = palette_color.hex_color.to_rgba_u32() {
                palette.push(RgbColor::from_rgba_u32(rgba_u32));
            }
        }

        Ok(CanvasConfig::new(default_world.id, palette))
    }
}

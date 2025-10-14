use crate::error::{AppError, AppResult};
use crate::infrastructure_config::ColorPaletteConfig;
use crate::ports::outgoing::world_store::DynWorldStorePort;
use domain::canvas_config::CanvasConfig;
use std::sync::Arc;

pub struct CanvasConfigService {
    world_store: DynWorldStorePort,
    color_palette_config: Arc<ColorPaletteConfig>,
}

impl CanvasConfigService {
    pub fn new(
        world_store: DynWorldStorePort,
        color_palette_config: Arc<ColorPaletteConfig>,
    ) -> Self {
        Self {
            world_store,
            color_palette_config,
        }
    }

    pub async fn get_canvas_config(&self) -> AppResult<CanvasConfig> {
        let default_world =
            self.world_store
                .get_default_world()
                .await?
                .ok_or_else(|| AppError::NotFound {
                    message: "No default world found".to_string(),
                })?;

        let palette = self.color_palette_config.colors.clone();
        let special_colors: Vec<(u8, String)> = self
            .color_palette_config
            .get_special_colors_with_ids()
            .into_iter()
            .map(|(id, purpose)| (id, purpose.clone()))
            .collect();

        Ok(CanvasConfig::new(default_world.id, palette, special_colors))
    }
}

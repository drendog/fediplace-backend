use crate::error::AppResult;
use domain::world::{PaletteColor, World, WorldId};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait WorldStorePort: Send + Sync {
    async fn get_world_by_id(&self, world_id: &WorldId) -> AppResult<Option<World>>;
    async fn get_world_by_name(&self, name: &str) -> AppResult<Option<World>>;
    async fn get_default_world(&self) -> AppResult<Option<World>>;
    async fn list_worlds(&self) -> AppResult<Vec<World>>;
    async fn create_world(&self, world: &World) -> AppResult<()>;
    async fn get_palette_colors(&self, world_id: &WorldId) -> AppResult<Vec<PaletteColor>>;
}

pub type DynWorldStorePort = Arc<dyn WorldStorePort>;

use std::sync::Arc;

use crate::error::AppResult;
use domain::world::{World, WorldId};

#[async_trait::async_trait]
pub trait WorldStorePort: Send + Sync {
    async fn get_world_by_id(&self, world_id: &WorldId) -> AppResult<Option<World>>;
    async fn get_world_by_name(&self, name: &str) -> AppResult<Option<World>>;
    async fn get_default_world(&self) -> AppResult<Option<World>>;
    async fn list_worlds(&self) -> AppResult<Vec<World>>;
    async fn create_world(&self, world: &World) -> AppResult<()>;
}

pub type DynWorldStorePort = Arc<dyn WorldStorePort>;

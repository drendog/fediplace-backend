use crate::{error::AppResult, ports::outgoing::world_store::DynWorldStorePort};
use domain::world::{World, WorldId};

pub struct WorldService {
    world_store: DynWorldStorePort,
}

impl WorldService {
    pub fn new(world_store: DynWorldStorePort) -> Self {
        Self { world_store }
    }

    pub async fn get_world_by_id(&self, world_id: &WorldId) -> AppResult<Option<World>> {
        self.world_store.get_world_by_id(world_id).await
    }

    pub async fn get_world_by_name(&self, name: &str) -> AppResult<Option<World>> {
        self.world_store.get_world_by_name(name).await
    }

    pub async fn get_default_world(&self) -> AppResult<Option<World>> {
        self.world_store.get_default_world().await
    }

    pub async fn list_worlds(&self) -> AppResult<Vec<World>> {
        self.world_store.list_worlds().await
    }

    pub async fn create_world(&self, name: String) -> AppResult<World> {
        let world = World::new(name);
        self.world_store.create_world(&world).await?;
        Ok(world)
    }
}

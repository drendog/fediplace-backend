use crate::{coords::TileCoord, world::WorldId};

#[derive(Clone, Debug)]
pub struct TileVersionEvent {
    pub world_id: WorldId,
    pub coord: TileCoord,
    pub version: u64,
}

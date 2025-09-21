use crate::coords::TileCoord;

#[derive(Clone, Debug)]
pub struct TileVersionEvent {
    pub coord: TileCoord,
    pub version: u64,
}

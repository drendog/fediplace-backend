use crate::auth::UserId;
use crate::color::ColorId;
use crate::coords::{GlobalCoord, PixelCoord, TileCoord};
use crate::world::WorldId;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintAction {
    pub world_id: WorldId,
    pub user_id: UserId,
    pub global_coord: GlobalCoord,
    pub color_id: ColorId,
    pub timestamp: OffsetDateTime,
}

impl PaintAction {
    #[must_use]
    pub fn tile_coord(&self, tile_size: usize) -> TileCoord {
        self.global_coord.to_tile_coord(tile_size)
    }

    #[must_use]
    pub fn pixel_coord(&self, tile_size: usize) -> PixelCoord {
        self.global_coord.to_pixel_coord(tile_size)
    }

    #[must_use]
    pub fn from_tile_and_pixel(
        world_id: WorldId,
        user_id: UserId,
        tile_coord: TileCoord,
        pixel_coord: PixelCoord,
        color_id: ColorId,
        timestamp: OffsetDateTime,
        tile_size: usize,
    ) -> Self {
        Self {
            world_id,
            user_id,
            global_coord: GlobalCoord::from_tile_and_pixel(tile_coord, pixel_coord, tile_size),
            color_id,
            timestamp,
        }
    }
}

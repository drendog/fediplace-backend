use crate::color::{ColorId, HexColor};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorldId(pub Uuid);

impl WorldId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WorldId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct World {
    pub id: WorldId,
    pub name: String,
    pub is_default: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl World {
    pub fn new(name: String) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            id: WorldId::new(),
            name,
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaletteColor {
    pub id: Uuid,
    pub world_id: WorldId,
    pub palette_index: i16,
    pub hex_color: HexColor,
}

impl PaletteColor {
    pub fn new(id: Uuid, world_id: WorldId, palette_index: i16, hex_color: HexColor) -> Self {
        Self {
            id,
            world_id,
            palette_index,
            hex_color,
        }
    }

    pub fn color_id(&self) -> ColorId {
        ColorId::new(self.palette_index)
    }
}

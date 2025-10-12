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

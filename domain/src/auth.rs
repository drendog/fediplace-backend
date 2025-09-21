use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

impl UserId {
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

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct UserPublic {
    pub id: UserId,
    pub email: String,
    pub username: String,
    pub email_verified_at: Option<time::OffsetDateTime>,
    pub available_charges: i32,
    pub charges_updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthProvider {
    Google,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub provider: String,
    pub provider_user_id: String,
}

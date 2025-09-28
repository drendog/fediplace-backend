use uuid::Uuid;

use crate::auth::UserId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BanId(pub Uuid);

impl BanId {
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

impl Default for BanId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Ban {
    pub id: BanId,
    pub user_id: UserId,
    pub banned_by_user_id: Option<UserId>,
    pub reason: String,
    pub banned_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
}

impl Ban {
    pub fn new(
        user_id: UserId,
        banned_by_user_id: Option<UserId>,
        reason: String,
        expires_at: Option<time::OffsetDateTime>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            id: BanId::new(),
            user_id,
            banned_by_user_id,
            reason,
            banned_at: now,
            expires_at,
            created_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => expires_at > time::OffsetDateTime::now_utc(),
            None => true,
        }
    }

    pub fn is_permanent(&self) -> bool {
        self.expires_at.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct BanRequest {
    pub user_id: UserId,
    pub reason: String,
    pub expires_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BanError {
    #[error("User is already banned")]
    UserAlreadyBanned,
    #[error("Ban not found")]
    BanNotFound,
    #[error("Cannot ban admin user")]
    CannotBanAdmin,
    #[error("Only admins can ban users")]
    InsufficientPermissions,
    #[error("Invalid ban duration")]
    InvalidDuration,
}
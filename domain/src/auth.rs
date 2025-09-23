use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RoleId(pub Uuid);

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

impl RoleId {
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

impl Default for RoleId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RoleType {
    Admin,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleType::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(RoleType::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl Role {
    pub fn role_type(&self) -> Option<RoleType> {
        RoleType::from_str(&self.name)
    }
}

#[derive(Debug, Clone)]
pub struct UserRole {
    pub user_id: UserId,
    pub role_id: RoleId,
    pub assigned_at: time::OffsetDateTime,
    pub assigned_by: Option<UserId>,
}

#[derive(Debug, Clone)]
pub struct UserPublic {
    pub id: UserId,
    pub email: String,
    pub username: String,
    pub email_verified_at: Option<time::OffsetDateTime>,
    pub available_charges: i32,
    pub charges_updated_at: time::OffsetDateTime,
    pub roles: Vec<Role>,
}

impl UserPublic {
    pub fn has_role(&self, role_name: &str) -> bool {
        self.roles.iter().any(|role| role.name == role_name)
    }

    pub fn has_role_type(&self, role_type: RoleType) -> bool {
        self.roles.iter().any(|role| role.role_type() == Some(role_type))
    }

    pub fn is_admin(&self) -> bool {
        self.has_role_type(RoleType::Admin)
    }
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

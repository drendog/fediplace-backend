use axum_login::{AuthUser, AuthnBackend, UserId as AxumUserId};
use domain::auth::{Role, RoleType, UserId, UserPublic};
use fedi_wplace_application::ports::outgoing::{
    password_hasher::DynPasswordHasherPort, user_store::DynUserStorePort,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use fedi_wplace_application::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub email_verified_at: Option<time::OffsetDateTime>,
    pub roles: Vec<Role>,
}

impl From<UserPublic> for User {
    fn from(user_public: UserPublic) -> Self {
        Self {
            id: *user_public.id.as_uuid(),
            email: user_public.email,
            username: user_public.username,
            email_verified_at: user_public.email_verified_at,
            roles: user_public.roles,
        }
    }
}

impl From<User> for UserPublic {
    fn from(user: User) -> Self {
        Self {
            id: UserId::from_uuid(user.id),
            email: user.email,
            username: user.username,
            email_verified_at: user.email_verified_at,
            available_charges: 0,
            charges_updated_at: time::OffsetDateTime::now_utc(),
            roles: user.roles,
        }
    }
}

impl User {
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

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.email.as_bytes()
    }
}

#[derive(Clone)]
pub struct AuthBackend {
    user_store: DynUserStorePort,
    password_hasher: DynPasswordHasherPort,
}

impl AuthBackend {
    pub fn new(user_store: DynUserStorePort, password_hasher: DynPasswordHasherPort) -> Self {
        Self {
            user_store,
            password_hasher,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = AppError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user_data = self
            .user_store
            .find_user_by_email(&creds.email)
            .await
            .map_err(|_| AppError::InternalServerError)?;

        let Some((user_id, _email, _username, password_hash, _email_verified_at)) = user_data
        else {
            return Ok(None);
        };

        let Some(ref stored_hash) = password_hash else {
            return Ok(None);
        };

        let password_valid = self
            .password_hasher
            .verify(&creds.password, stored_hash)
            .map_err(|_| AppError::InternalServerError)?;

        if password_valid {
            let user_public = self
                .user_store
                .find_user_by_id(user_id)
                .await
                .map_err(|_| AppError::InternalServerError)?;

            if let Some(user_public) = user_public {
                Ok(Some(User::from(user_public)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn get_user(
        &self,
        user_id: &AxumUserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = self
            .user_store
            .find_user_by_id(*user_id)
            .await
            .map_err(|_| AppError::InternalServerError)?;

        Ok(user.map(User::from))
    }
}

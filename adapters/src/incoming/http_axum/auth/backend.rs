use axum_login::{AuthUser, AuthnBackend, UserId as AxumUserId};
use domain::auth::{UserId, UserPublic};
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
}

impl From<UserPublic> for User {
    fn from(user_public: UserPublic) -> Self {
        Self {
            id: *user_public.id.as_uuid(),
            email: user_public.email,
            username: user_public.username,
            email_verified_at: user_public.email_verified_at,
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
        }
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

        let Some((user_id, email, username, password_hash, email_verified_at)) = user_data else {
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
            Ok(Some(User {
                id: user_id,
                email,
                username,
                email_verified_at,
            }))
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

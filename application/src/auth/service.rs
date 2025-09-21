use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::password_validator::PasswordValidator;
use crate::error::{AppError, AppResult};
use crate::ports::incoming::auth::AuthUseCase;
use crate::ports::outgoing::email_sender::DynEmailSenderPort;
use crate::ports::outgoing::password_hasher::PasswordHasherPort;
use crate::ports::outgoing::user_store::UserStorePort;
use domain::auth::UserPublic;

pub struct AuthService {
    user_store: Arc<dyn UserStorePort>,
    password_hasher: Arc<dyn PasswordHasherPort>,
    email_sender: DynEmailSenderPort,
    password_validator: PasswordValidator,
}

impl AuthService {
    pub fn new(
        user_store: Arc<dyn UserStorePort>,
        password_hasher: Arc<dyn PasswordHasherPort>,
        email_sender: DynEmailSenderPort,
    ) -> Self {
        Self {
            user_store,
            password_hasher,
            email_sender,
            password_validator: PasswordValidator::new(),
        }
    }
}

#[async_trait::async_trait]
impl AuthUseCase for AuthService {
    async fn register_local(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> AppResult<UserPublic> {
        self.password_validator.validate(&password)?;

        if (self.user_store.find_user_by_email(&email).await?).is_some() {
            return Err(AppError::ValidationError {
                message: "User with this email already exists".to_string(),
            });
        }
        if (self.user_store.find_user_by_username(&username).await?).is_some() {
            return Err(AppError::ValidationError {
                message: "Username already exists".to_string(),
            });
        }

        let password_hash = self.password_hasher.hash(&password)?;

        let user = self
            .user_store
            .create_user_with_password(&email, &username, &password_hash)
            .await?;

        let verification_token = Uuid::new_v4().to_string().replace('-', "");

        let expires_at = OffsetDateTime::now_utc() + Duration::hours(24);

        self.user_store
            .store_verification_token(*user.id.as_uuid(), &verification_token, expires_at)
            .await?;

        self.email_sender
            .send_verification_email(&email, &username, &verification_token)
            .await?;

        Ok(user)
    }

    async fn verify_email(&self, token: String) -> AppResult<UserPublic> {
        self.user_store.verify_user_by_token(&token).await
    }

    async fn login_local(&self, email: String, password: String) -> AppResult<UserPublic> {
        let user_data = self
            .user_store
            .find_user_by_email(&email)
            .await?
            .ok_or_else(|| AppError::ValidationError {
                message: "Invalid email or password".to_string(),
            })?;

        let (user_id, _stored_email, _stored_username, password_hash, _email_verified_at) =
            user_data;
        let password_hash = password_hash.ok_or_else(|| AppError::ValidationError {
            message: "User account does not support local login".to_string(),
        })?;

        if !self.password_hasher.verify(&password, &password_hash)? {
            return Err(AppError::ValidationError {
                message: "Invalid email or password".to_string(),
            });
        }

        let user_public = self
            .user_store
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::InternalServerError)?;

        Ok(user_public)
    }

    async fn logout(&self) -> AppResult<()> {
        Ok(())
    }

    async fn me(&self, user_id: Uuid) -> AppResult<UserPublic> {
        self.user_store
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::ValidationError {
                message: "User not found".to_string(),
            })
    }

    async fn upsert_social_identity(
        &self,
        provider: String,
        provider_user_id: String,
        email: Option<String>,
        username: Option<String>,
    ) -> AppResult<UserPublic> {
        self.user_store
            .create_or_get_social_user(
                &provider,
                &provider_user_id,
                email.as_deref(),
                username.as_deref(),
            )
            .await
    }

    async fn update_username(&self, user_id: Uuid, new_username: String) -> AppResult<UserPublic> {
        if new_username.trim().is_empty() {
            return Err(AppError::ValidationError {
                message: "Username cannot be empty".to_string(),
            });
        }

        if new_username.len() > 32 {
            return Err(AppError::ValidationError {
                message: "Username must be 32 characters or less".to_string(),
            });
        }

        if let Some(_existing_user) = self.user_store.find_user_by_username(&new_username).await? {
            return Err(AppError::ValidationError {
                message: "Username already exists".to_string(),
            });
        }

        self.user_store
            .update_username(user_id, &new_username)
            .await
    }
}

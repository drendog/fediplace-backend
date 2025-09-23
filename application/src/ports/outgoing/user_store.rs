use std::sync::Arc;

use crate::error::AppResult;
use domain::auth::UserPublic;
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait UserStorePort: Send + Sync {
    async fn create_user_with_password(
        &self,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> AppResult<UserPublic>;
    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> AppResult<Option<(Uuid, String, String, Option<String>, Option<OffsetDateTime>)>>;
    async fn find_user_by_username(&self, username: &str) -> AppResult<Option<UserPublic>>;
    async fn find_user_by_id(&self, id: Uuid) -> AppResult<Option<UserPublic>>;
    async fn create_or_get_social_user(
        &self,
        provider: &str,
        provider_user_id: &str,
        email: Option<&str>,
        username: Option<&str>,
    ) -> AppResult<UserPublic>;
    async fn store_verification_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_at: OffsetDateTime,
    ) -> AppResult<()>;
    async fn verify_user_by_token(&self, token: &str) -> AppResult<UserPublic>;
    async fn update_username(&self, user_id: Uuid, new_username: &str) -> AppResult<UserPublic>;
    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        assigned_by: Uuid,
    ) -> AppResult<UserPublic>;
}

pub type DynUserStorePort = Arc<dyn UserStorePort>;

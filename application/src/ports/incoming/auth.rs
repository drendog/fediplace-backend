use crate::error::AppResult;
use domain::auth::UserPublic;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait AuthUseCase: Send + Sync {
    async fn register_local(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> AppResult<UserPublic>;
    async fn verify_email(&self, token: String) -> AppResult<UserPublic>;
    async fn login_local(&self, email: String, password: String) -> AppResult<UserPublic>;
    async fn logout(&self) -> AppResult<()>;
    async fn me(&self, user_id: Uuid) -> AppResult<UserPublic>;
    async fn upsert_social_identity(
        &self,
        provider: String,
        provider_user_id: String,
        email: Option<String>,
        username: Option<String>,
    ) -> AppResult<UserPublic>;
    async fn update_username(&self, user_id: Uuid, new_username: String) -> AppResult<UserPublic>;
}

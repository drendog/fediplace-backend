use crate::error::AppResult;
use domain::{auth::UserId, ban::Ban};
use time::OffsetDateTime;

#[async_trait::async_trait]
pub trait BanUseCase: Send + Sync {
    async fn ban_user(
        &self,
        user_id: UserId,
        banned_by_user_id: UserId,
        reason: String,
        expires_at: Option<OffsetDateTime>,
    ) -> AppResult<()>;

    async fn check_user_ban_status(&self, user_id: &UserId) -> AppResult<Option<Ban>>;

    async fn unban_user(&self, user_id: UserId, unbanned_by: UserId) -> AppResult<()>;

    async fn get_active_bans(&self, requesting_user_id: UserId) -> AppResult<Vec<Ban>>;
}

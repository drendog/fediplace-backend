use std::sync::Arc;

use crate::error::AppResult;
use domain::{auth::UserId, ban::Ban};

#[async_trait::async_trait]
pub trait BanStorePort: Send + Sync {
    async fn create_ban(&self, ban: &Ban) -> AppResult<()>;

    async fn get_active_ban_by_user_id(&self, user_id: &UserId) -> AppResult<Option<Ban>>;

    async fn remove_ban_by_user_id(&self, user_id: &UserId) -> AppResult<()>;

    async fn get_all_active_bans(&self) -> AppResult<Vec<Ban>>;

    async fn remove_user_pixels(&self, user_id: &UserId) -> AppResult<u64>;
}

pub type DynBanStorePort = Arc<dyn BanStorePort>;
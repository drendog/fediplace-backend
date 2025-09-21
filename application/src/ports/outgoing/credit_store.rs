use std::sync::Arc;

use crate::error::AppResult;
use domain::auth::UserId;
use domain::credits::{CreditBalance, CreditConfig};

#[async_trait::async_trait]
pub trait CreditStorePort: Send + Sync {
    async fn get_user_credits(&self, user_id: &UserId) -> AppResult<CreditBalance>;
    async fn update_user_credits(&self, user_id: &UserId, balance: &CreditBalance)
    -> AppResult<()>;
    async fn spend_user_credits(
        &self,
        user_id: &UserId,
        cost: i32,
        config: &CreditConfig,
    ) -> AppResult<CreditBalance>;
}

pub type DynCreditStorePort = Arc<dyn CreditStorePort>;

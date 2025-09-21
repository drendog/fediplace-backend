use std::sync::Arc;

use crate::error::AppResult;

#[async_trait::async_trait]
pub trait EmailSenderPort: Send + Sync {
    async fn send_verification_email(
        &self,
        recipient_email: &str,
        username: &str,
        verification_token: &str,
    ) -> AppResult<()>;
}

pub type DynEmailSenderPort = Arc<dyn EmailSenderPort>;

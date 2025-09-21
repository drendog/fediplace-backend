use std::sync::Arc;

use crate::error::AppResult;

#[async_trait::async_trait]
pub trait PasswordHasherPort: Send + Sync {
    fn hash(&self, password: &str) -> AppResult<String>;
    fn verify(&self, password: &str, password_hash: &str) -> AppResult<bool>;
}

pub type DynPasswordHasherPort = Arc<dyn PasswordHasherPort>;

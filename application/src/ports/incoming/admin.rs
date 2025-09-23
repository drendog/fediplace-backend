use crate::error::AppResult;
use domain::auth::UserPublic;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait AdminUseCase: Send + Sync {
    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        assigned_by: Uuid,
    ) -> AppResult<UserPublic>;
}

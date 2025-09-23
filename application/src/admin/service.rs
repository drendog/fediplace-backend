use std::sync::Arc;
use uuid::Uuid;

use crate::{
    error::AppResult,
    ports::{incoming::admin::AdminUseCase, outgoing::user_store::DynUserStorePort},
};
use domain::auth::UserPublic;

pub struct AdminService {
    user_store: DynUserStorePort,
}

impl AdminService {
    pub fn new(user_store: DynUserStorePort) -> Self {
        Self { user_store }
    }
}

#[async_trait::async_trait]
impl AdminUseCase for AdminService {
    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        assigned_by: Uuid,
    ) -> AppResult<UserPublic> {
        self.user_store
            .assign_role_to_user(user_id, role_id, assigned_by)
            .await
    }
}

pub type DynAdminUseCase = Arc<dyn AdminUseCase>;

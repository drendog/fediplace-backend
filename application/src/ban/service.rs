use std::sync::Arc;
use time::OffsetDateTime;

use crate::error::{AppError, AppResult};
use crate::ports::incoming::ban::BanUseCase;
use crate::ports::outgoing::ban_store::BanStorePort;
use crate::ports::outgoing::user_store::UserStorePort;
use domain::{
    auth::{RoleType, UserId},
    ban::{Ban, BanError},
};

pub struct BanService {
    ban_store: Arc<dyn BanStorePort>,
    user_store: Arc<dyn UserStorePort>,
}

impl BanService {
    pub fn new(ban_store: Arc<dyn BanStorePort>, user_store: Arc<dyn UserStorePort>) -> Self {
        Self {
            ban_store,
            user_store,
        }
    }

    async fn validate_ban_permissions(&self, banned_by_user_id: &UserId) -> AppResult<()> {
        let user = self
            .user_store
            .find_user_by_id(*banned_by_user_id.as_uuid())
            .await?
            .ok_or_else(|| AppError::ValidationError {
                message: "Admin user not found".to_string(),
            })?;

        if !user.has_role_type(RoleType::Admin) {
            return Err(AppError::Forbidden);
        }

        Ok(())
    }

    async fn validate_target_user(&self, user_id: &UserId) -> AppResult<()> {
        let user = self
            .user_store
            .find_user_by_id(*user_id.as_uuid())
            .await?
            .ok_or_else(|| AppError::ValidationError {
                message: "User to ban not found".to_string(),
            })?;

        if user.has_role_type(RoleType::Admin) {
            return Err(AppError::Forbidden);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl BanUseCase for BanService {
    async fn ban_user(
        &self,
        user_id: UserId,
        banned_by_user_id: UserId,
        reason: String,
        expires_at: Option<OffsetDateTime>,
    ) -> AppResult<()> {
        self.validate_ban_permissions(&banned_by_user_id).await?;
        self.validate_target_user(&user_id).await?;

        if let Some(existing_ban) = self.ban_store.get_active_ban_by_user_id(&user_id).await? {
            if existing_ban.is_active() {
                return Err(AppError::ValidationError {
                    message: BanError::UserAlreadyBanned.to_string(),
                });
            }
        }

        if let Some(expires) = expires_at {
            if expires <= OffsetDateTime::now_utc() {
                return Err(AppError::ValidationError {
                    message: BanError::InvalidDuration.to_string(),
                });
            }
        }

        let ban = Ban::new(
            user_id.clone(),
            Some(banned_by_user_id.clone()),
            reason,
            expires_at,
        );

        self.ban_store.create_ban(&ban).await?;
        let pixels_removed = self.ban_store.remove_user_pixels(&user_id).await?;

        tracing::info!(
            user_id = %user_id.as_uuid(),
            banned_by = %banned_by_user_id.as_uuid(),
            pixels_removed = pixels_removed,
            "User banned and pixels removed"
        );

        Ok(())
    }

    async fn check_user_ban_status(&self, user_id: &UserId) -> AppResult<Option<Ban>> {
        let ban = self.ban_store.get_active_ban_by_user_id(user_id).await?;

        match ban {
            Some(ban) if ban.is_active() => Ok(Some(ban)),
            _ => Ok(None),
        }
    }

    async fn unban_user(&self, user_id: UserId, unbanned_by: UserId) -> AppResult<()> {
        self.validate_ban_permissions(&unbanned_by).await?;

        let ban = self.ban_store.get_active_ban_by_user_id(&user_id).await?;
        if ban.is_none() || !ban.as_ref().is_some_and(Ban::is_active) {
            return Err(AppError::ValidationError {
                message: BanError::BanNotFound.to_string(),
            });
        }

        self.ban_store.remove_ban_by_user_id(&user_id).await?;

        tracing::info!(
            user_id = %user_id.as_uuid(),
            unbanned_by = %unbanned_by.as_uuid(),
            "User unbanned"
        );

        Ok(())
    }

    async fn get_active_bans(&self, requesting_user_id: UserId) -> AppResult<Vec<Ban>> {
        self.validate_ban_permissions(&requesting_user_id).await?;

        let all_bans = self.ban_store.get_all_active_bans().await?;

        Ok(all_bans.into_iter().filter(Ban::is_active).collect())
    }
}

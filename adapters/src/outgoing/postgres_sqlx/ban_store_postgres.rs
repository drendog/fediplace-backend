use sqlx::{PgPool, types::time::OffsetDateTime};
use tracing::instrument;

use domain::{
    auth::UserId,
    ban::{Ban, BanId},
};
use fedi_wplace_application::{error::AppResult, ports::outgoing::ban_store::BanStorePort};

use super::utils::PostgresExecutor;

pub struct PostgresBanStoreAdapter {
    pool: PgPool,
    executor: PostgresExecutor,
}

impl PostgresBanStoreAdapter {
    pub fn new(pool: PgPool, query_timeout_secs: u64) -> Self {
        Self {
            pool,
            executor: PostgresExecutor::new(query_timeout_secs),
        }
    }
}

#[async_trait::async_trait]
impl BanStorePort for PostgresBanStoreAdapter {
    #[instrument(skip(self, ban))]
    async fn create_ban(&self, ban: &Ban) -> AppResult<()> {
        self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    INSERT INTO banned_users (id, user_id, banned_by_user_id, reason, banned_at, expires_at, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (user_id) DO UPDATE SET
                        banned_by_user_id = EXCLUDED.banned_by_user_id,
                        reason = EXCLUDED.reason,
                        banned_at = EXCLUDED.banned_at,
                        expires_at = EXCLUDED.expires_at,
                        created_at = EXCLUDED.created_at
                    "#,
                    ban.id.as_uuid(),
                    ban.user_id.as_uuid(),
                    ban.banned_by_user_id.as_ref().map(UserId::as_uuid),
                    ban.reason,
                    OffsetDateTime::from(ban.banned_at),
                    ban.expires_at.map(OffsetDateTime::from),
                    OffsetDateTime::from(ban.created_at)
                )
                .execute(&self.pool)
            },
            "Failed to create ban",

        )
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_active_ban_by_user_id(&self, user_id: &UserId) -> AppResult<Option<Ban>> {
        let result = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT id, user_id, banned_by_user_id, reason, banned_at, expires_at, created_at
                    FROM banned_users
                    WHERE user_id = $1
                    "#,
                        user_id.as_uuid()
                    )
                    .fetch_optional(&self.pool)
                },
                &format!("Failed to get ban for user {}", user_id.as_uuid()),
            )
            .await?;

        match result {
            Some(row) => {
                let ban = Ban {
                    id: BanId::from_uuid(row.id),
                    user_id: UserId::from_uuid(row.user_id),
                    banned_by_user_id: row.banned_by_user_id.map(UserId::from_uuid),
                    reason: row.reason,
                    banned_at: row.banned_at,
                    expires_at: row.expires_at,
                    created_at: row.created_at,
                };
                Ok(Some(ban))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn remove_ban_by_user_id(&self, user_id: &UserId) -> AppResult<()> {
        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    DELETE FROM banned_users
                    WHERE user_id = $1
                    "#,
                        user_id.as_uuid()
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to remove ban for user {}", user_id.as_uuid()),
            )
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_all_active_bans(&self) -> AppResult<Vec<Ban>> {
        let results = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT id, user_id, banned_by_user_id, reason, banned_at, expires_at, created_at
                    FROM banned_users
                    ORDER BY banned_at DESC
                    "#
                    )
                    .fetch_all(&self.pool)
                },
                "Failed to get all active bans",
            )
            .await?;

        let bans = results
            .into_iter()
            .map(|row| Ban {
                id: BanId::from_uuid(row.id),
                user_id: UserId::from_uuid(row.user_id),
                banned_by_user_id: row.banned_by_user_id.map(UserId::from_uuid),
                reason: row.reason,
                banned_at: row.banned_at,
                expires_at: row.expires_at,
                created_at: row.created_at,
            })
            .collect();

        Ok(bans)
    }

    #[instrument(skip(self))]
    async fn remove_user_pixels(&self, user_id: &UserId) -> AppResult<u64> {
        let result = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    DELETE FROM pixel_history
                    WHERE user_id = $1
                    "#,
                        user_id.as_uuid()
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to remove pixels for user {}", user_id.as_uuid()),
            )
            .await?;

        Ok(result.rows_affected())
    }
}

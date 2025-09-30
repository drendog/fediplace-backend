use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use tracing::{debug, instrument};

use domain::auth::UserId;
use domain::credits::{CreditBalance, CreditConfig};
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::credit_store::CreditStorePort,
};

use super::utils::PostgresExecutor;

pub struct PostgresCreditStoreAdapter {
    pool: PgPool,
    executor: PostgresExecutor,
}

impl PostgresCreditStoreAdapter {
    pub fn new(pool: PgPool, query_timeout_secs: u64) -> Self {
        Self {
            pool,
            executor: PostgresExecutor::new(query_timeout_secs),
        }
    }
}

#[async_trait::async_trait]
impl CreditStorePort for PostgresCreditStoreAdapter {
    #[instrument(skip(self))]
    async fn get_user_credits(&self, user_id: &UserId) -> AppResult<CreditBalance> {
        let row = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query(
                        r"
                    SELECT available_charges, charges_updated_at
                    FROM users
                    WHERE id = $1
                    ",
                    )
                    .bind(user_id.as_uuid())
                    .fetch_optional(&self.pool)
                },
                &format!("Failed to get credits for user {}", user_id.as_uuid()),
            )
            .await?;

        if let Some(record) = row {
            let available_charges: i32 =
                record
                    .try_get("available_charges")
                    .map_err(|e| AppError::DatabaseError {
                        message: format!("Failed to get available_charges: {}", e),
                    })?;
            let charges_updated_at: OffsetDateTime =
                record
                    .try_get("charges_updated_at")
                    .map_err(|e| AppError::DatabaseError {
                        message: format!("Failed to get charges_updated_at: {}", e),
                    })?;

            Ok(CreditBalance::new(available_charges, charges_updated_at))
        } else {
            Err(AppError::DatabaseError {
                message: "User not found".to_string(),
            })
        }
    }

    #[instrument(skip(self, balance))]
    async fn update_user_credits(
        &self,
        user_id: &UserId,
        balance: &CreditBalance,
    ) -> AppResult<()> {
        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query(
                        r"
                    UPDATE users
                    SET available_charges = $1, charges_updated_at = $2
                    WHERE id = $3
                    ",
                    )
                    .bind(balance.available_charges)
                    .bind(balance.charges_updated_at)
                    .bind(user_id.as_uuid())
                    .execute(&self.pool)
                },
                &format!("Failed to update credits for user {}", user_id.as_uuid()),
            )
            .await?;

        debug!(
            "Updated credits for user {} to {} at {}",
            user_id.as_uuid(),
            balance.available_charges,
            balance.charges_updated_at
        );

        Ok(())
    }

    #[instrument(skip(self, config))]
    async fn spend_user_credits(
        &self,
        user_id: &UserId,
        cost: i32,
        config: &CreditConfig,
    ) -> AppResult<CreditBalance> {
        let mut balance = self.get_user_credits(user_id).await?;
        let now = OffsetDateTime::now_utc();

        balance
            .spend_charges(cost, now, config)
            .map_err(|err| AppError::InsufficientCredits {
                message: format!(
                    "Required {} credits, but only {} available",
                    err.required, err.available
                ),
            })?;

        self.update_user_credits(user_id, &balance).await?;

        debug!(
            "Spent {} credits for user {}, {} remaining",
            cost,
            user_id.as_uuid(),
            balance.available_charges
        );

        Ok(balance)
    }
}

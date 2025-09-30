use fedi_wplace_application::error::{AppError, AppResult};
use sqlx::{PgPool, Postgres, Transaction};
use std::{future::Future, time::Duration};
use tokio::time::timeout;

pub struct PostgresExecutor {
    timeout_secs: u64,
}

impl PostgresExecutor {
    pub fn new(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    pub async fn execute_with_timeout<T, F, Fut>(
        &self,
        operation: F,
        error_context: &str,
    ) -> AppResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, sqlx::Error>>,
    {
        timeout(Duration::from_secs(self.timeout_secs), operation())
            .await
            .map_err(|_| AppError::DatabaseError {
                message: "DB timeout".to_string(),
            })?
            .map_err(|e| AppError::DatabaseError {
                message: format!("{}: {}", error_context, e),
            })
    }
}

pub async fn begin_transaction(pool: &PgPool) -> AppResult<Transaction<'_, Postgres>> {
    pool.begin().await.map_err(|e| AppError::DatabaseError {
        message: format!("Failed to begin transaction: {}", e),
    })
}

pub async fn commit_transaction(tx: Transaction<'_, Postgres>) -> AppResult<()> {
    tx.commit().await.map_err(|e| AppError::DatabaseError {
        message: format!("Failed to commit transaction: {}", e),
    })
}

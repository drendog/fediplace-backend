use domain::{
    action::PaintAction,
    coords::{GlobalCoord, TileCoord},
};
use sqlx::{PgPool, types::time::OffsetDateTime};
use std::{future::Future, time::Duration};
use tokio::time::timeout;
use tracing::instrument;
use uuid::Uuid;

use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::pixel_history_store::{PixelHistoryEntry, PixelHistoryStorePort, PixelInfo},
};

pub struct PostgresPixelHistoryStoreAdapter {
    pool: PgPool,
    tile_size: usize,
}

impl PostgresPixelHistoryStoreAdapter {
    pub fn new(pool: PgPool, tile_size: usize) -> Self {
        Self { pool, tile_size }
    }

    async fn execute_with_timeout<T, F, Fut>(
        &self,
        operation: F,
        error_context: &str,
    ) -> AppResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, sqlx::Error>>,
    {
        timeout(Duration::from_secs(5), operation())
            .await
            .map_err(|_| AppError::DatabaseError {
                message: "DB timeout".to_string(),
            })?
            .map_err(|e| AppError::DatabaseError {
                message: format!("{}: {}", error_context, e),
            })
    }
}

#[async_trait::async_trait]
impl PixelHistoryStorePort for PostgresPixelHistoryStoreAdapter {
    #[instrument(skip(self, actions))]
    async fn record_paint_actions(&self, actions: &[PaintAction]) -> AppResult<()> {
        if actions.is_empty() {
            return Ok(());
        }

        let mut user_ids: Vec<Uuid> = Vec::with_capacity(actions.len());
        let mut global_xs: Vec<i32> = Vec::with_capacity(actions.len());
        let mut global_ys: Vec<i32> = Vec::with_capacity(actions.len());
        let mut color_ids: Vec<i16> = Vec::with_capacity(actions.len());
        let mut timestamps: Vec<OffsetDateTime> = Vec::with_capacity(actions.len());

        for action in actions {
            user_ids.push(action.user_id.0);
            global_xs.push(action.global_coord.x);
            global_ys.push(action.global_coord.y);
            color_ids.push(i16::from(action.color_id.0));
            timestamps.push(action.timestamp);
        }

        self.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    INSERT INTO pixel_history (user_id, global_x, global_y, color_id, created_at)
                    SELECT * FROM UNNEST($1::UUID[], $2::INTEGER[], $3::INTEGER[], $4::SMALLINT[], $5::TIMESTAMPTZ[])
                    ON CONFLICT (global_x, global_y)
                    DO UPDATE SET
                        user_id = EXCLUDED.user_id,
                        color_id = EXCLUDED.color_id,
                        created_at = EXCLUDED.created_at
                    "#,
                    &user_ids[..],
                    &global_xs[..],
                    &global_ys[..],
                    &color_ids[..],
                    &timestamps[..]
                )
                .execute(&self.pool)
            },
            &format!("Failed to record {} paint actions", actions.len()),
        )
        .await?;

        tracing::debug!("Successfully recorded {} paint actions", actions.len());
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_history_for_tile(&self, coord: TileCoord) -> AppResult<Vec<PixelHistoryEntry>> {
        let tile_size = self.tile_size as i32;
        let min_x = coord.x * tile_size;
        let max_x = min_x + tile_size - 1;
        let min_y = coord.y * tile_size;
        let max_y = min_y + tile_size - 1;

        let history = self
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT
                            ph.user_id,
                            u.username,
                            ph.global_x,
                            ph.global_y,
                            ph.color_id,
                            ph.created_at
                        FROM pixel_history ph
                        JOIN users u ON ph.user_id = u.id
                        WHERE ph.global_x >= $1 AND ph.global_x <= $2
                          AND ph.global_y >= $3 AND ph.global_y <= $4
                        ORDER BY ph.created_at DESC
                        "#,
                        min_x,
                        max_x,
                        min_y,
                        max_y
                    )
                    .fetch_all(&self.pool)
                },
                &format!(
                    "Failed to get pixel history for tile ({}, {})",
                    coord.x, coord.y
                ),
            )
            .await?;

        let history_entries: Vec<PixelHistoryEntry> = history
            .into_iter()
            .map(|row| {
                let pixel_x = (row.global_x - min_x) as usize;
                let pixel_y = (row.global_y - min_y) as usize;

                PixelHistoryEntry {
                    user_id: row.user_id,
                    username: row.username,
                    pixel_x,
                    pixel_y,
                    color_id: row.color_id as u8,
                    timestamp: row.created_at,
                }
            })
            .collect();

        tracing::debug!(
            "Retrieved {} history entries for tile ({}, {})",
            history_entries.len(),
            coord.x,
            coord.y
        );
        Ok(history_entries)
    }

    #[instrument(skip(self))]
    async fn get_current_tile_state(&self, coord: TileCoord) -> AppResult<Vec<(usize, usize, u8)>> {
        let tile_size = self.tile_size as i32;
        let min_x = coord.x * tile_size;
        let max_x = min_x + tile_size - 1;
        let min_y = coord.y * tile_size;
        let max_y = min_y + tile_size - 1;

        let pixels = self
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT
                            global_x,
                            global_y,
                            color_id
                        FROM pixel_history
                        WHERE global_x >= $1 AND global_x <= $2
                          AND global_y >= $3 AND global_y <= $4
                        "#,
                        min_x,
                        max_x,
                        min_y,
                        max_y
                    )
                    .fetch_all(&self.pool)
                },
                &format!(
                    "Failed to get current tile state for ({}, {})",
                    coord.x, coord.y
                ),
            )
            .await?;

        let current_state: Vec<(usize, usize, u8)> = pixels
            .into_iter()
            .map(|row| {
                let pixel_x = (row.global_x - min_x) as usize;
                let pixel_y = (row.global_y - min_y) as usize;
                (pixel_x, pixel_y, row.color_id as u8)
            })
            .collect();

        tracing::debug!(
            "Retrieved {} pixels for tile ({}, {})",
            current_state.len(),
            coord.x,
            coord.y
        );
        Ok(current_state)
    }

    #[instrument(skip(self))]
    async fn get_distinct_tile_count(&self, tile_size: usize) -> AppResult<i64> {
        let tile_size = tile_size as i32;

        let result = self
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT COUNT(DISTINCT (global_x / $1, global_y / $1)) as count
                        FROM pixel_history
                        "#,
                        tile_size
                    )
                    .fetch_one(&self.pool)
                },
                "Failed to get distinct tile count",
            )
            .await?;

        let count = result.count.unwrap_or(0);
        tracing::debug!("Retrieved distinct tile count: {}", count);
        Ok(count)
    }

    #[instrument(skip(self))]
    async fn get_pixel_info(&self, coord: GlobalCoord) -> AppResult<Option<PixelInfo>> {
        let pixel_info = self
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT
                            ph.user_id,
                            u.username,
                            ph.color_id,
                            ph.created_at
                        FROM pixel_history ph
                        JOIN users u ON ph.user_id = u.id
                        WHERE ph.global_x = $1 AND ph.global_y = $2
                        "#,
                        coord.x,
                        coord.y
                    )
                    .fetch_optional(&self.pool)
                },
                &format!(
                    "Failed to get pixel info for coordinates ({}, {})",
                    coord.x, coord.y
                ),
            )
            .await?;

        let result = pixel_info.map(|row| PixelInfo {
            user_id: row.user_id,
            username: row.username,
            color_id: row.color_id as u8,
            timestamp: row.created_at,
        });

        tracing::debug!(
            "Retrieved pixel info for coordinates ({}, {}): {}",
            coord.x,
            coord.y,
            result.is_some()
        );
        Ok(result)
    }
}

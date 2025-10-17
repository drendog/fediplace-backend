use domain::{
    action::PaintAction,
    coords::{GlobalCoord, TileCoord},
    world::WorldId,
};
use sqlx::{PgPool, types::time::OffsetDateTime};
use std::collections::HashMap;
use tracing::instrument;
use uuid::Uuid;

use fedi_wplace_application::{
    error::AppResult,
    ports::outgoing::pixel_history_store::{PixelHistoryEntry, PixelHistoryStorePort, PixelInfo},
};

use super::utils::PostgresExecutor;

pub struct PostgresPixelHistoryStoreAdapter {
    pool: PgPool,
    tile_size: usize,
    executor: PostgresExecutor,
}

impl PostgresPixelHistoryStoreAdapter {
    pub fn new(pool: PgPool, tile_size: usize, query_timeout_secs: u64) -> Self {
        Self {
            pool,
            tile_size,
            executor: PostgresExecutor::new(query_timeout_secs),
        }
    }
}

#[async_trait::async_trait]
impl PixelHistoryStorePort for PostgresPixelHistoryStoreAdapter {
    #[instrument(skip(self, actions))]
    async fn record_paint_actions(
        &self,
        world_id: &WorldId,
        actions: &[PaintAction],
    ) -> AppResult<()> {
        if actions.is_empty() {
            return Ok(());
        }

        let world_uuid = world_id.as_uuid();

        let palette_map = self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    SELECT palette_index, id FROM palette_colors
                    WHERE world_id = $1
                    "#,
                    world_uuid
                )
                .fetch_all(&self.pool)
            },
            "Failed to fetch palette color mappings",
        )
        .await?
        .into_iter()
        .map(|row| (row.palette_index, row.id))
        .collect::<HashMap<i16, Uuid>>();

        let mut world_ids: Vec<Uuid> = Vec::with_capacity(actions.len());
        let mut user_ids: Vec<Uuid> = Vec::with_capacity(actions.len());
        let mut global_xs: Vec<i32> = Vec::with_capacity(actions.len());
        let mut global_ys: Vec<i32> = Vec::with_capacity(actions.len());
        let mut color_ids: Vec<Option<Uuid>> = Vec::with_capacity(actions.len());
        let mut timestamps: Vec<OffsetDateTime> = Vec::with_capacity(actions.len());

        for action in actions {
            world_ids.push(*world_uuid);
            user_ids.push(*action.user_id.as_uuid());
            global_xs.push(action.global_coord.x);
            global_ys.push(action.global_coord.y);

            let color_uuid = if action.color_id.is_transparent() {
                None
            } else {
                palette_map.get(&action.color_id.id()).copied()
            };

            color_ids.push(color_uuid);
            timestamps.push(action.timestamp);
        }

        self.executor.execute_with_timeout(
            || {
                sqlx::query!(
                    r#"
                    INSERT INTO pixel_history (world_id, user_id, global_x, global_y, color_id, created_at)
                    SELECT * FROM UNNEST($1::UUID[], $2::UUID[], $3::INTEGER[], $4::INTEGER[], $5::UUID[], $6::TIMESTAMPTZ[])
                    ON CONFLICT (world_id, global_x, global_y)
                    DO UPDATE SET
                        user_id = EXCLUDED.user_id,
                        color_id = EXCLUDED.color_id,
                        created_at = EXCLUDED.created_at
                    "#,
                    &world_ids[..],
                    &user_ids[..],
                    &global_xs[..],
                    &global_ys[..],
                    &color_ids as &[Option<Uuid>],
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
    async fn get_history_for_tile(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<Vec<PixelHistoryEntry>> {
        let world_uuid = world_id.as_uuid();
        let tile_size = self.tile_size as i32;
        let min_x = coord.x * tile_size;
        let max_x = min_x + tile_size - 1;
        let min_y = coord.y * tile_size;
        let max_y = min_y + tile_size - 1;

        let history = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT
                        ph.user_id,
                        u.username,
                        ph.global_x,
                        ph.global_y,
                        COALESCE(pc.palette_index, -1) as "palette_index!",
                        ph.created_at
                    FROM pixel_history ph
                    JOIN users u ON ph.user_id = u.id
                    LEFT JOIN palette_colors pc ON ph.color_id = pc.id
                    WHERE ph.world_id = $1
                      AND ph.global_x >= $2 AND ph.global_x <= $3
                      AND ph.global_y >= $4 AND ph.global_y <= $5
                    ORDER BY ph.created_at DESC
                    "#,
                        world_uuid,
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
                    color_id: row.palette_index as i16,
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
    async fn get_current_tile_state(
        &self,
        world_id: &WorldId,
        coord: TileCoord,
    ) -> AppResult<Vec<(usize, usize, i16)>> {
        let world_uuid = world_id.as_uuid();
        let tile_size = self.tile_size as i32;
        let min_x = coord.x * tile_size;
        let max_x = min_x + tile_size - 1;
        let min_y = coord.y * tile_size;
        let max_y = min_y + tile_size - 1;

        let pixels = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT
                        ph.global_x,
                        ph.global_y,
                        COALESCE(pc.palette_index, -1) as "palette_index!"
                    FROM pixel_history ph
                    LEFT JOIN palette_colors pc ON ph.color_id = pc.id
                    WHERE ph.world_id = $1
                      AND ph.global_x >= $2 AND ph.global_x <= $3
                      AND ph.global_y >= $4 AND ph.global_y <= $5
                    "#,
                        world_uuid,
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

        let current_state: Vec<(usize, usize, i16)> = pixels
            .into_iter()
            .map(|row| {
                let pixel_x = (row.global_x - min_x) as usize;
                let pixel_y = (row.global_y - min_y) as usize;
                (pixel_x, pixel_y, row.palette_index as i16)
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
    async fn get_distinct_tile_count(
        &self,
        world_id: &WorldId,
        tile_size: usize,
    ) -> AppResult<i64> {
        let world_uuid = world_id.as_uuid();
        let tile_size = tile_size as i32;

        let result = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT COUNT(DISTINCT (global_x / $1, global_y / $1)) as count
                    FROM pixel_history
                    WHERE world_id = $2
                    "#,
                        tile_size,
                        world_uuid
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
    async fn get_pixel_info(
        &self,
        world_id: &WorldId,
        coord: GlobalCoord,
    ) -> AppResult<Option<PixelInfo>> {
        let world_uuid = world_id.as_uuid();
        let pixel_info = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                    SELECT
                        ph.user_id,
                        u.username,
                        COALESCE(pc.palette_index, -1) as "palette_index!",
                        ph.created_at
                    FROM pixel_history ph
                    JOIN users u ON ph.user_id = u.id
                    LEFT JOIN palette_colors pc ON ph.color_id = pc.id
                    WHERE ph.world_id = $1 AND ph.global_x = $2 AND ph.global_y = $3
                    "#,
                        world_uuid,
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
            color_id: row.palette_index as i16,
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

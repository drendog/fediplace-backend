use super::utils::PostgresExecutor;
use domain::{
    color::HexColor,
    world::{PaletteColor, World, WorldId},
};
use fedi_wplace_application::{error::AppResult, ports::outgoing::world_store::WorldStorePort};
use sqlx::PgPool;
use tracing::instrument;

pub struct PostgresWorldStoreAdapter {
    pool: PgPool,
    executor: PostgresExecutor,
}

impl PostgresWorldStoreAdapter {
    pub fn new(pool: PgPool, query_timeout_secs: u64) -> Self {
        Self {
            pool,
            executor: PostgresExecutor::new(query_timeout_secs),
        }
    }
}

#[async_trait::async_trait]
impl WorldStorePort for PostgresWorldStoreAdapter {
    #[instrument(skip(self))]
    async fn get_world_by_id(&self, world_id: &WorldId) -> AppResult<Option<World>> {
        let world_uuid = world_id.as_uuid();

        let row = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT id, name, is_default, created_at, updated_at
                        FROM worlds
                        WHERE id = $1
                        "#,
                        world_uuid
                    )
                    .fetch_optional(&self.pool)
                },
                &format!("Failed to get world by id {}", world_uuid),
            )
            .await?;

        Ok(row.map(|r| World {
            id: WorldId::from_uuid(r.id),
            name: r.name,
            is_default: r.is_default,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    #[instrument(skip(self))]
    async fn get_world_by_name(&self, name: &str) -> AppResult<Option<World>> {
        let row = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT id, name, is_default, created_at, updated_at
                        FROM worlds
                        WHERE name = $1
                        "#,
                        name
                    )
                    .fetch_optional(&self.pool)
                },
                &format!("Failed to get world by name {}", name),
            )
            .await?;

        Ok(row.map(|r| World {
            id: WorldId::from_uuid(r.id),
            name: r.name,
            is_default: r.is_default,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    #[instrument(skip(self))]
    async fn get_default_world(&self) -> AppResult<Option<World>> {
        let row = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT id, name, is_default, created_at, updated_at
                        FROM worlds
                        WHERE is_default = TRUE
                        "#
                    )
                    .fetch_optional(&self.pool)
                },
                "Failed to get default world",
            )
            .await?;

        Ok(row.map(|r| World {
            id: WorldId::from_uuid(r.id),
            name: r.name,
            is_default: r.is_default,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    #[instrument(skip(self))]
    async fn list_worlds(&self) -> AppResult<Vec<World>> {
        let rows = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT id, name, is_default, created_at, updated_at
                        FROM worlds
                        ORDER BY created_at DESC
                        "#
                    )
                    .fetch_all(&self.pool)
                },
                "Failed to list worlds",
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| World {
                id: WorldId::from_uuid(r.id),
                name: r.name,
                is_default: r.is_default,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    #[instrument(skip(self, world))]
    async fn create_world(&self, world: &World) -> AppResult<()> {
        let world_id = world.id.as_uuid();

        self.executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        INSERT INTO worlds (id, name, is_default, created_at, updated_at)
                        VALUES ($1, $2, $3, $4, $5)
                        "#,
                        world_id,
                        world.name,
                        world.is_default,
                        world.created_at,
                        world.updated_at
                    )
                    .execute(&self.pool)
                },
                &format!("Failed to create world {}", world.name),
            )
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_palette_colors(&self, world_id: &WorldId) -> AppResult<Vec<PaletteColor>> {
        let world_uuid = world_id.as_uuid();

        let rows = self
            .executor
            .execute_with_timeout(
                || {
                    sqlx::query!(
                        r#"
                        SELECT id, world_id, palette_index, hex_color
                        FROM palette_colors
                        WHERE world_id = $1
                        ORDER BY palette_index
                        "#,
                        world_uuid
                    )
                    .fetch_all(&self.pool)
                },
                &format!("Failed to get palette colors for world {}", world_uuid),
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| PaletteColor {
                id: r.id,
                world_id: WorldId::from_uuid(r.world_id),
                palette_index: r.palette_index,
                hex_color: HexColor::new(r.hex_color),
            })
            .collect())
    }
}

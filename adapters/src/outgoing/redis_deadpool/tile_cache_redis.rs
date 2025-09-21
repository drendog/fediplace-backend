use deadpool_redis::{
    Connection as RedisConnection, Pool as RedisPool,
    redis::{AsyncCommands, cmd},
};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

use domain::coords::TileCoord;
use fedi_wplace_application::{
    error::{AppError, AppResult},
    ports::outgoing::tile_cache::TileCachePort,
};

use super::keys::RedisKeyBuilder;

#[derive(Clone)]
pub struct RedisTileCacheConfig {
    pub namespace_env: String,
    pub ttl_current: u64,
    pub ttl_webp: u64,
    pub ttl_rgba: u64,
    pub ttl_missing: u64,
}

#[derive(Clone)]
struct CacheTtls {
    current: u64,
    webp: u64,
    rgba: u64,
    missing: u64,
}

pub struct RedisTileCacheAdapter {
    redis_pool: RedisPool,
    redis_keys: RedisKeyBuilder,
    ttls: CacheTtls,
}

impl RedisTileCacheAdapter {
    pub fn new(
        redis_pool: RedisPool,
        namespace_env: &str,
        ttl_current: u64,
        ttl_webp: u64,
        ttl_rgba: u64,
        ttl_missing: u64,
    ) -> Self {
        let redis_keys = RedisKeyBuilder::new(namespace_env);
        let ttls = CacheTtls {
            current: ttl_current,
            webp: ttl_webp,
            rgba: ttl_rgba,
            missing: ttl_missing,
        };
        Self {
            redis_pool,
            redis_keys,
            ttls,
        }
    }

    pub async fn get_redis_connection(&self) -> AppResult<RedisConnection> {
        match timeout(Duration::from_millis(1000), self.redis_pool.get()).await {
            Ok(conn) => conn.map_err(|e| AppError::CacheError {
                message: format!("Failed to get Redis connection: {}", e),
            }),
            Err(_) => Err(AppError::CacheError {
                message: "Redis connection timeout".to_string(),
            }),
        }
    }

    pub fn redis_keys(&self) -> &RedisKeyBuilder {
        &self.redis_keys
    }
}

#[async_trait::async_trait]
impl TileCachePort for RedisTileCacheAdapter {
    async fn get_version(&self, coord: TileCoord) -> AppResult<Option<u64>> {
        let mut conn = self.get_redis_connection().await?;
        let current_key = self.redis_keys.current_key(coord.x, coord.y);

        match conn.get::<_, u64>(&current_key).await {
            Ok(version) => {
                debug!("Found version {} in Redis for tile {}", version, coord);
                Ok(Some(version))
            }
            Err(_) => Ok(None),
        }
    }

    async fn get_palette(&self, coord: TileCoord, version: u64) -> AppResult<Option<Vec<u8>>> {
        let mut conn = self.get_redis_connection().await?;
        let palette_key = self.redis_keys.palette_key(coord.x, coord.y, version);

        match conn.get::<_, Vec<u8>>(&palette_key).await {
            Ok(palette_bytes) if !palette_bytes.is_empty() => {
                debug!("Palette cache hit for tile {} v{}", coord, version);
                Ok(Some(palette_bytes))
            }
            _ => {
                debug!("Palette cache miss for tile {} v{}", coord, version);
                Ok(None)
            }
        }
    }

    async fn store_palette(&self, coord: TileCoord, version: u64, data: &[u8]) -> AppResult<()> {
        let mut conn = self.get_redis_connection().await?;
        let palette_key = self.redis_keys.palette_key(coord.x, coord.y, version);

        let _: () = conn
            .set_ex(&palette_key, data, self.ttls.rgba)
            .await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to store palette data for tile {}: {}", coord, e),
            })?;

        debug!("Stored palette data for tile {} v{}", coord, version);
        Ok(())
    }

    async fn get_webp(&self, coord: TileCoord, version: u64) -> AppResult<Option<Vec<u8>>> {
        let mut conn = self.get_redis_connection().await?;
        let webp_key = self.redis_keys.webp_key(coord.x, coord.y, version);

        match conn.get::<_, Vec<u8>>(&webp_key).await {
            Ok(webp_bytes) if !webp_bytes.is_empty() => {
                debug!("WebP cache hit for tile {} v{}", coord, version);
                Ok(Some(webp_bytes))
            }
            _ => {
                debug!("WebP cache miss for tile {} v{}", coord, version);
                Ok(None)
            }
        }
    }

    async fn store_webp(&self, coord: TileCoord, version: u64, data: &[u8]) -> AppResult<()> {
        if data.is_empty() {
            warn!(
                "Refusing to cache empty WebP data for tile {} v{}",
                coord, version
            );
            return Ok(());
        }

        let mut conn = self.get_redis_connection().await?;
        let webp_key = self.redis_keys.webp_key(coord.x, coord.y, version);

        let _: () = conn
            .set_ex(&webp_key, data, self.ttls.webp)
            .await
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to store WebP data for tile {}: {}", coord, e),
            })?;

        debug!(
            "Stored WebP data for tile {} v{} ({} bytes)",
            coord,
            version,
            data.len()
        );
        Ok(())
    }

    async fn has_missing_sentinel(&self, coord: TileCoord) -> AppResult<bool> {
        let mut conn = self.get_redis_connection().await?;
        let missing_sentinel_key = self.redis_keys.missing_sentinel_key(coord.x, coord.y);

        match conn.get::<_, bool>(&missing_sentinel_key).await {
            Ok(exists) => Ok(exists),
            Err(_) => Ok(false),
        }
    }

    async fn set_missing_sentinel(&self, coord: TileCoord) -> AppResult<()> {
        let mut conn = self.get_redis_connection().await?;
        let missing_sentinel_key = self.redis_keys.missing_sentinel_key(coord.x, coord.y);

        let _: () = conn
            .set_ex(&missing_sentinel_key, true, self.ttls.missing)
            .await
            .map_err(|e| {
                warn!("Failed to set missing sentinel for tile {}: {}", coord, e);
                AppError::CacheError {
                    message: format!("Failed to set missing sentinel for tile {}: {}", coord, e),
                }
            })?;

        debug!("Set missing sentinel for tile {}", coord);
        Ok(())
    }

    async fn clear_missing_sentinel(&self, coord: TileCoord) -> AppResult<()> {
        let mut conn = self.get_redis_connection().await?;
        let missing_sentinel_key = self.redis_keys.missing_sentinel_key(coord.x, coord.y);

        let _: () = conn.del(&missing_sentinel_key).await.map_err(|e| {
            warn!("Failed to clear missing sentinel for tile {}: {}", coord, e);
            AppError::CacheError {
                message: format!("Failed to clear missing sentinel for tile {}: {}", coord, e),
            }
        })?;

        debug!("Cleared missing sentinel for tile {}", coord);
        Ok(())
    }

    async fn update_version_optimistically(&self, coord: TileCoord, version: u64) {
        if let Ok(mut conn) = self.get_redis_connection().await {
            let current_key = self.redis_keys.current_key(coord.x, coord.y);
            let _: () = conn
                .set_ex(&current_key, version, self.ttls.current)
                .await
                .unwrap_or(());
        }
    }

    async fn store_palette_optimistically(&self, coord: TileCoord, version: u64, data: &[u8]) {
        if let Ok(mut conn) = self.get_redis_connection().await {
            let palette_key = self.redis_keys.palette_key(coord.x, coord.y, version);
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&palette_key, data, self.ttls.rgba)
                .await
            {
                warn!(
                    "Failed to store palette data for tile {} v{}: {}",
                    coord, version, e
                );
            }
        }
    }

    async fn clear_cache(&self) -> AppResult<()> {
        use tracing::info;

        let mut conn = self.get_redis_connection().await?;
        let namespace_prefix = self.redis_keys.namespace_prefix();

        let mut cursor: u64 = 0;
        let mut redis_key_count = 0;
        let mut affected_keys = Vec::new();

        loop {
            let (next, batch): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&namespace_prefix)
                .query_async(&mut *conn)
                .await
                .map_err(|e| AppError::CacheError {
                    message: format!("Failed to scan cache keys: {}", e),
                })?;

            if !batch.is_empty() {
                redis_key_count += batch.len();
                affected_keys.extend(batch.clone());
                conn.del::<_, ()>(&batch)
                    .await
                    .map_err(|e| AppError::CacheError {
                        message: format!("Failed to clear cache: {}", e),
                    })?;
            }

            if next == 0 {
                break;
            }
            cursor = next;
        }

        if affected_keys.is_empty() {
            info!(
                "No Redis cache keys found with prefix '{}'",
                namespace_prefix
            );
        } else {
            info!(
                "Cleared {} Redis cache keys with prefix '{}': {:?}",
                redis_key_count, namespace_prefix, affected_keys
            );
        }
        Ok(())
    }
}

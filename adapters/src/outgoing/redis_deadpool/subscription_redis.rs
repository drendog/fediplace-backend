use std::{net::IpAddr, string::ToString as StdToString, sync::LazyLock, time::Duration};

use deadpool_redis::{
    Connection as RedisConnection, Pool as RedisPool,
    redis::{
        ErrorKind, FromRedisValue, RedisError, RedisResult, Script, Value as RedisValue, cmd, pipe,
    },
};
use tokio::time::timeout;
use tracing::debug;

use crate::shared::net::ip_key;
use domain::coords::TileCoord;
use fedi_wplace_application::{
    contracts::subscriptions::{SubscriptionRejection, SubscriptionResult},
    error::{AppError, AppResult},
    ports::outgoing::subscription_port::SubscriptionPort,
};

static FIFO_SUBSCRIPTION_WITH_EVICTION_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
        local subscription_zset = KEYS[1]
        local refcount_hash = KEYS[2]
        local max_tiles = tonumber(ARGV[1])
        local ttl_ms = tonumber(ARGV[2])
        local requested_tile = ARGV[3]

        local redis_time = redis.call('TIME')
        local current_timestamp_ms = redis_time[1] * 1000 + math.floor(redis_time[2] / 1000)

        redis.call('ZREMRANGEBYSCORE', subscription_zset, 0, current_timestamp_ms)

        local new_refcount = redis.call('HINCRBY', refcount_hash, requested_tile, 1)
        local evicted_tile_key = ''

        if new_refcount == 1 then
            local current_subscription_count = redis.call('ZCARD', subscription_zset)
            if current_subscription_count >= max_tiles then
                local oldest_subscriptions = redis.call('ZRANGE', subscription_zset, 0, 0)
                if #oldest_subscriptions > 0 then
                    evicted_tile_key = oldest_subscriptions[1]
                    redis.call('ZREM', subscription_zset, evicted_tile_key)
                    local evicted_refcount = redis.call('HINCRBY', refcount_hash, evicted_tile_key, -1)
                    if evicted_refcount <= 0 then
                        redis.call('HDEL', refcount_hash, evicted_tile_key)
                    end
                end
            end
            local expiration_timestamp = current_timestamp_ms + ttl_ms
            redis.call('ZADD', subscription_zset, expiration_timestamp, requested_tile)
            return {1, 'new', redis.call('ZCARD', subscription_zset), evicted_tile_key}
        else
            return {1, 'already', redis.call('ZCARD', subscription_zset), ''}
        end
        ",
    )
});

static REFCOUNT_AWARE_UNSUBSCRIBE_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
        local subscription_zset = KEYS[1]
        local refcount_hash = KEYS[2]
        local tile_to_unsubscribe = ARGV[1]

        local remaining_refcount = redis.call('HINCRBY', refcount_hash, tile_to_unsubscribe, -1)

        if remaining_refcount <= 0 then
            redis.call('HDEL', refcount_hash, tile_to_unsubscribe)
            local removed_from_zset = redis.call('ZREM', subscription_zset, tile_to_unsubscribe)
            return {removed_from_zset, 0}
        else
            return {0, remaining_refcount}
        end
        ",
    )
});

#[derive(Clone)]
pub struct RedisSubscriptionConfig {
    pub max_tiles_per_ip: usize,
    pub subscription_ttl_ms: u64,
}

#[derive(Clone)]
pub struct RedisSubscriptionAdapter {
    redis_pool: RedisPool,
    policy: SubscriptionPolicyConfig,
}

impl RedisSubscriptionAdapter {
    pub fn new(redis_pool: RedisPool, max_tiles_per_ip: usize, subscription_ttl_ms: u64) -> Self {
        let policy = SubscriptionPolicyConfig {
            max_tiles_per_ip,
            ttl_ms: subscription_ttl_ms,
        };
        Self { redis_pool, policy }
    }

    fn subscription_policy(&self) -> &SubscriptionPolicyConfig {
        &self.policy
    }
}

#[derive(Debug, Clone)]
struct SubscriptionPolicyConfig {
    max_tiles_per_ip: usize,
    ttl_ms: u64,
}

#[derive(Debug)]
struct SubscriptionScriptResult {
    accepted: bool,
    status: String,
    count: usize,
    evicted_tile: Option<String>,
}

impl FromRedisValue for SubscriptionScriptResult {
    fn from_redis_value(v: &RedisValue) -> RedisResult<Self> {
        if let RedisValue::Array(values) = v {
            if values.len() != 4 {
                return Err((ErrorKind::TypeError, "Expected array of 4 elements").into());
            }

            let accepted = i64::from_redis_value(values.first().ok_or(RedisError::from((
                ErrorKind::TypeError,
                "Missing value at index 0",
            )))?)?
                == 1;
            let status = String::from_redis_value(values.get(1).ok_or(RedisError::from((
                ErrorKind::TypeError,
                "Missing value at index 1",
            )))?)?;
            let count = i64::from_redis_value(values.get(2).ok_or(RedisError::from((
                ErrorKind::TypeError,
                "Missing value at index 2",
            )))?)? as usize;
            let evicted_str = String::from_redis_value(values.get(3).ok_or(RedisError::from(
                (ErrorKind::TypeError, "Missing value at index 3"),
            ))?)?;
            let evicted_tile = if evicted_str.is_empty() {
                None
            } else {
                Some(evicted_str)
            };

            Ok(SubscriptionScriptResult {
                accepted,
                status,
                count,
                evicted_tile,
            })
        } else {
            Err((ErrorKind::TypeError, "Expected array").into())
        }
    }
}

#[derive(Debug)]
struct UnsubscribeScriptResult {
    removed: bool,
    remaining_refcount: i64,
}

impl FromRedisValue for UnsubscribeScriptResult {
    fn from_redis_value(v: &RedisValue) -> RedisResult<Self> {
        if let RedisValue::Array(values) = v {
            if values.len() != 2 {
                return Err((ErrorKind::TypeError, "Expected array of 2 elements").into());
            }

            let removed = i64::from_redis_value(values.first().ok_or(RedisError::from((
                ErrorKind::TypeError,
                "Missing value at index 0",
            )))?)?
                > 0;
            let remaining_refcount = i64::from_redis_value(values.get(1).ok_or(
                RedisError::from((ErrorKind::TypeError, "Missing value at index 1")),
            )?)?;

            Ok(UnsubscribeScriptResult {
                removed,
                remaining_refcount,
            })
        } else {
            Err((ErrorKind::TypeError, "Expected array").into())
        }
    }
}

fn refcount_tracking_key_for_ip(ip_subscription_key: &str) -> String {
    format!("{}:cnt", ip_subscription_key)
}

async fn subscribe_with_fifo_eviction_awareness(
    redis_connection: &mut RedisConnection,
    subscription_policy: &SubscriptionPolicyConfig,
    ip_subscription_key: &str,
    tile_coordinate_key: &str,
) -> AppResult<(bool, String, usize, Option<String>)> {
    let refcount_key = refcount_tracking_key_for_ip(ip_subscription_key);

    let result: SubscriptionScriptResult = FIFO_SUBSCRIPTION_WITH_EVICTION_SCRIPT
        .key(ip_subscription_key)
        .key(&refcount_key)
        .arg(subscription_policy.max_tiles_per_ip)
        .arg(subscription_policy.ttl_ms)
        .arg(tile_coordinate_key)
        .invoke_async(redis_connection)
        .await
        .map_err(|redis_error| AppError::CacheError {
            message: format!("Failed to execute subscribe script: {}", redis_error),
        })?;

    debug!(
        "Subscribe result: success={}, reason={}, count={}, evicted={:?}",
        result.accepted, result.status, result.count, result.evicted_tile
    );

    Ok((
        result.accepted,
        result.status,
        result.count,
        result.evicted_tile,
    ))
}

async fn unsubscribe_with_refcount_cleanup(
    redis_connection: &mut RedisConnection,
    ip_subscription_key: &str,
    tile_coordinate_key: &str,
) -> AppResult<bool> {
    let refcount_key = refcount_tracking_key_for_ip(ip_subscription_key);

    let result: UnsubscribeScriptResult = REFCOUNT_AWARE_UNSUBSCRIBE_SCRIPT
        .key(ip_subscription_key)
        .key(&refcount_key)
        .arg(tile_coordinate_key)
        .invoke_async(redis_connection)
        .await
        .map_err(|redis_error| AppError::CacheError {
            message: format!("Failed to execute unsubscribe script: {}", redis_error),
        })?;

    debug!(
        "Unsubscribe {} -> removed={}, remaining_count={}",
        tile_coordinate_key, result.removed, result.remaining_refcount
    );
    Ok(result.removed)
}

async fn refresh_subscription_expiration_times(
    redis_connection: &mut RedisConnection,
    subscription_policy: &SubscriptionPolicyConfig,
    ip_subscription_key: &str,
    tile_coordinate_keys: &[String],
) -> AppResult<()> {
    if tile_coordinate_keys.is_empty() {
        return Ok(());
    }

    let redis_time_response: Vec<i64> =
        cmd("TIME")
            .query_async(redis_connection)
            .await
            .map_err(|redis_error| AppError::CacheError {
                message: format!("Failed to get Redis time: {}", redis_error),
            })?;

    if redis_time_response.len() != 2 {
        return Err(AppError::CacheError {
            message: "Invalid TIME response".to_string(),
        });
    }

    #[allow(clippy::indexing_slicing)] // safe because we checked redis_time_response.len() == 2
    let current_timestamp_ms = redis_time_response[0] * 1000 + redis_time_response[1] / 1000;
    let new_expiration_timestamp = current_timestamp_ms + subscription_policy.ttl_ms as i64;

    let mut redis_pipeline = pipe();
    redis_pipeline.atomic();

    redis_pipeline
        .cmd("ZREMRANGEBYSCORE")
        .arg(ip_subscription_key)
        .arg(0)
        .arg(current_timestamp_ms);

    for tile_key in tile_coordinate_keys {
        redis_pipeline.zadd(ip_subscription_key, tile_key, new_expiration_timestamp);
    }

    redis_pipeline
        .query_async::<()>(redis_connection)
        .await
        .map_err(|redis_error| AppError::CacheError {
            message: format!("Failed to refresh subscriptions: {}", redis_error),
        })?;

    debug!(
        "Refreshed {} subscriptions for key {}",
        tile_coordinate_keys.len(),
        ip_subscription_key
    );
    Ok(())
}

#[async_trait::async_trait]
impl SubscriptionPort for RedisSubscriptionAdapter {
    async fn subscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<SubscriptionResult> {
        let policy = self.subscription_policy();
        let ip_key = ip_key(ip);

        let redis_connection_timeout = Duration::from_millis(500);
        let mut redis_conn = timeout(redis_connection_timeout, self.redis_pool.get())
            .await
            .map_err(|_| AppError::CacheError {
                message: "Redis connection timeout".to_string(),
            })?
            .map_err(|redis_error| AppError::CacheError {
                message: format!("Failed to get Redis connection: {}", redis_error),
            })?;

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for tile_coord in tiles {
            let tile_key = tile_coord.to_string();

            match subscribe_with_fifo_eviction_awareness(
                &mut redis_conn,
                policy,
                &ip_key,
                &tile_key,
            )
            .await
            {
                Ok((subscription_accepted, subscription_reason, _active_count, maybe_evicted)) => {
                    if let Some(evicted_tile_key) = maybe_evicted {
                        if let Ok(evicted_coord) = evicted_tile_key.parse::<TileCoord>() {
                            rejected.push(SubscriptionRejection {
                                tile: evicted_coord,
                                reason: "Evicted due to FIFO policy".to_string(),
                            });
                        }
                    }

                    if subscription_accepted {
                        accepted.push(*tile_coord);
                    } else {
                        rejected.push(SubscriptionRejection {
                            tile: *tile_coord,
                            reason: if subscription_reason == "limit" {
                                "Subscription limit exceeded".to_string()
                            } else {
                                subscription_reason
                            },
                        });
                    }
                }
                Err(redis_error) => {
                    rejected.push(SubscriptionRejection {
                        tile: *tile_coord,
                        reason: format!("Redis error: {}", redis_error),
                    });
                }
            }
        }

        Ok(SubscriptionResult { accepted, rejected })
    }

    async fn unsubscribe(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<Vec<TileCoord>> {
        let ip_key = ip_key(ip);

        let redis_connection_timeout = Duration::from_millis(500);
        let mut redis_conn = timeout(redis_connection_timeout, self.redis_pool.get())
            .await
            .map_err(|_| AppError::CacheError {
                message: "Redis connection timeout".to_string(),
            })?
            .map_err(|redis_error| AppError::CacheError {
                message: format!("Failed to get Redis connection: {}", redis_error),
            })?;

        let mut unsubscribed = Vec::new();

        for tile_coord in tiles {
            let tile_key = tile_coord.to_string();

            match unsubscribe_with_refcount_cleanup(&mut redis_conn, &ip_key, &tile_key).await {
                Ok(was_removed) => {
                    if was_removed {
                        unsubscribed.push(*tile_coord);
                    }
                }
                Err(redis_error) => {
                    debug!(
                        "Failed to unsubscribe from tile {}: {}",
                        tile_coord, redis_error
                    );
                }
            }
        }

        Ok(unsubscribed)
    }

    async fn refresh_subscriptions(&self, ip: IpAddr, tiles: &[TileCoord]) -> AppResult<()> {
        if tiles.is_empty() {
            return Ok(());
        }

        let policy = self.subscription_policy();
        let ip_key = ip_key(ip);

        let redis_connection_timeout = Duration::from_millis(500);
        let mut redis_conn = timeout(redis_connection_timeout, self.redis_pool.get())
            .await
            .map_err(|_| AppError::CacheError {
                message: "Redis connection timeout".to_string(),
            })?
            .map_err(|redis_error| AppError::CacheError {
                message: format!("Failed to get Redis connection: {}", redis_error),
            })?;

        let tile_keys: Vec<String> = tiles.iter().map(StdToString::to_string).collect();

        refresh_subscription_expiration_times(&mut redis_conn, policy, &ip_key, &tile_keys).await
    }
}

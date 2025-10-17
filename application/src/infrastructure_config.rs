use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub tiles: TileConfig,
    pub db: DbConfig,
    pub redis: RedisConfig,
    pub websocket: WebSocketConfig,
    pub ws_policy: WsPolicyConfig,
    pub rate_limit: RateLimitConfig,
    pub credits: CreditConfig,
    pub logging: LoggingConfig,
    pub environment: EnvironmentConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub public_base_url: String,
    pub frontend_success_url: String,
    pub frontend_error_url: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_redirect_url: Option<String>,
    pub email: EmailConfig,
    pub argon2: Argon2Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2Config {
    pub memory_cost: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub output_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub email_backend: EmailBackend,
    pub smtp: SmtpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailBackend {
    #[serde(rename = "console")]
    Console,
    #[serde(rename = "smtp")]
    Smtp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileConfig {
    pub tile_size: usize,
    pub pixel_size: usize,
    pub buffer_pool_max_size: usize,
    pub cache_ttl: CacheTtlConfig,
    pub http_cache_control: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTtlConfig {
    pub redis_current_ttl_seconds: u64,
    pub redis_webp_ttl_seconds: u64,
    pub redis_rgba_ttl_seconds: u64,
    pub redis_missing_sentinel_ttl_seconds: u64,
    pub jitter_min_percent: u8,
    pub jitter_max_percent: u8,
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub database_url: SecretString,
    pub pool_size: u32,
    pub query_timeout_secs: u64,
}

impl Serialize for DbConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DbConfig", 3)?;
        state.serialize_field("database_url", "[REDACTED]")?;
        state.serialize_field("pool_size", &self.pool_size)?;
        state.serialize_field("query_timeout_secs", &self.query_timeout_secs)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for DbConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DbConfigHelper {
            database_url: String,
            pool_size: u32,
            #[serde(default = "default_query_timeout")]
            query_timeout_secs: u64,
        }

        fn default_query_timeout() -> u64 {
            5
        }

        let helper = DbConfigHelper::deserialize(deserializer)?;
        Ok(DbConfig {
            database_url: SecretString::from(helper.database_url),
            pool_size: helper.pool_size,
            query_timeout_secs: helper.query_timeout_secs,
        })
    }
}

impl DbConfig {
    #[must_use]
    pub fn redacted_url(&self) -> String {
        let url_str = self.database_url.expose_secret();
        match url::Url::parse(url_str) {
            Ok(mut url) => {
                if url.password().is_some() {
                    url.set_password(Some("***")).ok();
                }
                url.to_string()
            }
            Err(_) => "[INVALID_URL]".to_string(),
        }
    }

    #[must_use]
    pub fn database_url(&self) -> &str {
        self.database_url.expose_secret()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub redis_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub broadcast_buffer_size: usize,
    pub max_connections: Option<usize>,
    pub connection_buffer_size: usize,
    pub drop_newest_on_full_buffer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPolicyConfig {
    pub max_tiles_per_ip: usize,
    pub subscription_ttl_secs: u64,
    pub heartbeat_refresh_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub paint_requests_per_minute: u32,
    pub tile_requests_per_minute: u32,
    pub global_requests_per_minute: u32,
    pub websocket_messages_per_minute: u32,
    pub auth_requests_per_minute: u32,
    pub burst_size_multiplier: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditConfig {
    pub max_charges: i32,
    pub charge_cooldown_seconds: i32,
    pub initial_charges: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub include_location: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "pretty")]
    Pretty,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            email_backend: EmailBackend::Console,
            smtp: SmtpConfig::default(),
        }
    }
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: "noreply@example.com".to_string(),
            from_name: "FediPlace".to_string(),
            use_tls: true,
        }
    }
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory_cost: 19456,
            time_cost: 2,
            parallelism: 1,
            output_length: Some(32),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            cookie_name: "sid".to_string(),
            cookie_secure: false,
            public_base_url: "http://localhost:3000".to_string(),
            frontend_success_url: "http://localhost:3000/".to_string(),
            frontend_error_url: "http://localhost:3000/login?error=auth_failed".to_string(),
            google_client_id: None,
            google_client_secret: None,
            google_redirect_url: None,
            email: EmailConfig::default(),
            argon2: Argon2Config::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                cors_origin: None,
            },
            tiles: TileConfig {
                tile_size: 256,
                pixel_size: 1,
                buffer_pool_max_size: 16,
                http_cache_control: "public, max-age=5, must-revalidate".to_string(),
                cache_ttl: CacheTtlConfig {
                    redis_current_ttl_seconds: 600,
                    redis_webp_ttl_seconds: 3600,
                    redis_rgba_ttl_seconds: 14400,
                    redis_missing_sentinel_ttl_seconds: 45,
                    jitter_min_percent: 10,
                    jitter_max_percent: 20,
                },
            },
            db: DbConfig {
                database_url: SecretString::from("postgresql://localhost/fediplace"),
                pool_size: 10,
                query_timeout_secs: 5,
            },
            redis: RedisConfig {
                redis_url: "redis://localhost:6379".to_string(),
            },
            websocket: WebSocketConfig {
                broadcast_buffer_size: 1000,
                max_connections: None,
                connection_buffer_size: 100,
                drop_newest_on_full_buffer: false,
            },
            ws_policy: WsPolicyConfig {
                max_tiles_per_ip: 64,
                subscription_ttl_secs: 45,
                heartbeat_refresh_secs: 15,
            },
            rate_limit: RateLimitConfig {
                enabled: true,
                paint_requests_per_minute: 60,
                tile_requests_per_minute: 300,
                global_requests_per_minute: 1000,
                websocket_messages_per_minute: 120,
                auth_requests_per_minute: 30,
                burst_size_multiplier: 2,
            },
            credits: CreditConfig {
                max_charges: 30,
                charge_cooldown_seconds: 60,
                initial_charges: 30,
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: LogFormat::Pretty,
                include_location: false,
            },
            environment: EnvironmentConfig {
                env: "development".to_string(),
            },
            auth: AuthConfig::default(),
        }
    }
}

impl Config {
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> AppResult<()> {
        if self.tiles.tile_size == 0 || self.tiles.tile_size > 4096 {
            return Err(AppError::ConfigError {
                message: "tile_size must be between 1 and 4096".to_string(),
            });
        }

        if self.tiles.pixel_size == 0 || self.tiles.pixel_size > 32 {
            return Err(AppError::ConfigError {
                message: "pixel_size must be between 1 and 32".to_string(),
            });
        }

        if self.tiles.buffer_pool_max_size == 0 {
            return Err(AppError::ConfigError {
                message: "buffer_pool_max_size must be > 0".to_string(),
            });
        }

        if self.tiles.http_cache_control.trim().is_empty() {
            return Err(AppError::ConfigError {
                message: "http_cache_control cannot be empty".to_string(),
            });
        }

        if self.db.database_url.expose_secret().is_empty() {
            return Err(AppError::ConfigError {
                message: "database_url cannot be empty".to_string(),
            });
        }

        if self.db.pool_size == 0 {
            return Err(AppError::ConfigError {
                message: "db pool_size must be greater than 0".to_string(),
            });
        }

        if self.redis.redis_url.is_empty() {
            return Err(AppError::ConfigError {
                message: "redis_url cannot be empty".to_string(),
            });
        }

        if self.websocket.broadcast_buffer_size == 0 {
            return Err(AppError::ConfigError {
                message: "broadcast_buffer_size must be greater than 0".to_string(),
            });
        }

        if self.websocket.connection_buffer_size == 0 {
            return Err(AppError::ConfigError {
                message: "connection_buffer_size must be greater than 0".to_string(),
            });
        }

        if self.ws_policy.max_tiles_per_ip == 0 {
            return Err(AppError::ConfigError {
                message: "max_tiles_per_ip must be greater than 0".to_string(),
            });
        }

        if self.ws_policy.subscription_ttl_secs == 0 {
            return Err(AppError::ConfigError {
                message: "subscription_ttl_secs must be greater than 0".to_string(),
            });
        }

        if self.ws_policy.heartbeat_refresh_secs == 0 {
            return Err(AppError::ConfigError {
                message: "heartbeat_refresh_secs must be greater than 0".to_string(),
            });
        }

        if self.tiles.cache_ttl.jitter_min_percent > self.tiles.cache_ttl.jitter_max_percent {
            return Err(AppError::ConfigError {
                message: "jitter_min_percent must be <= jitter_max_percent".to_string(),
            });
        }

        if self.tiles.cache_ttl.jitter_max_percent > 100 {
            return Err(AppError::ConfigError {
                message: "jitter_max_percent must be <= 100".to_string(),
            });
        }

        if self.tiles.cache_ttl.redis_current_ttl_seconds == 0
            || self.tiles.cache_ttl.redis_webp_ttl_seconds == 0
            || self.tiles.cache_ttl.redis_rgba_ttl_seconds == 0
            || self.tiles.cache_ttl.redis_missing_sentinel_ttl_seconds == 0
        {
            return Err(AppError::ConfigError {
                message: "All TTL values must be greater than 0".to_string(),
            });
        }

        if self.rate_limit.enabled {
            if self.rate_limit.paint_requests_per_minute == 0
                || self.rate_limit.tile_requests_per_minute == 0
                || self.rate_limit.global_requests_per_minute == 0
                || self.rate_limit.websocket_messages_per_minute == 0
                || self.rate_limit.auth_requests_per_minute == 0
            {
                return Err(AppError::ConfigError {
                    message: "Rate limit values must be greater than 0 when enabled".to_string(),
                });
            }

            if self.rate_limit.burst_size_multiplier == 0 {
                return Err(AppError::ConfigError {
                    message: "burst_size_multiplier must be greater than 0".to_string(),
                });
            }
        }

        if self.credits.max_charges <= 0 {
            return Err(AppError::ConfigError {
                message: "max_charges must be greater than 0".to_string(),
            });
        }

        if self.credits.charge_cooldown_seconds <= 0 {
            return Err(AppError::ConfigError {
                message: "charge_cooldown_seconds must be greater than 0".to_string(),
            });
        }

        if self.credits.initial_charges < 0 {
            return Err(AppError::ConfigError {
                message: "initial_charges must be greater than or equal to 0".to_string(),
            });
        }

        if self.credits.initial_charges > self.credits.max_charges {
            return Err(AppError::ConfigError {
                message: "initial_charges cannot exceed max_charges".to_string(),
            });
        }

        if self.auth.argon2.memory_cost < 1024 {
            return Err(AppError::ConfigError {
                message: "Argon2 memory_cost must be at least 1024 KiB".to_string(),
            });
        }

        if self.auth.argon2.time_cost == 0 {
            return Err(AppError::ConfigError {
                message: "Argon2 time_cost must be greater than 0".to_string(),
            });
        }

        if self.auth.argon2.parallelism == 0 {
            return Err(AppError::ConfigError {
                message: "Argon2 parallelism must be greater than 0".to_string(),
            });
        }

        if let Some(output_len) = self.auth.argon2.output_length {
            if !(16..=512).contains(&output_len) {
                return Err(AppError::ConfigError {
                    message: "Argon2 output_length must be between 16 and 512 bytes".to_string(),
                });
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    #[must_use]
    pub fn ttl_with_jitter(&self, base_seconds: u64) -> u64 {
        use rand::Rng;
        let jitter_config = &self.tiles.cache_ttl;

        let min_percent = f64::from(jitter_config.jitter_min_percent) / 100.0;
        let max_percent = f64::from(jitter_config.jitter_max_percent) / 100.0;

        let mut rng = rand::rng();
        let jitter_factor = rng.random_range((1.0 + min_percent)..=(1.0 + max_percent));

        #[allow(clippy::cast_precision_loss)]
        let result = (base_seconds as f64 * jitter_factor).round();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let result_u64 = result as u64;
        result_u64
    }

    #[must_use]
    pub fn redis_current_ttl_with_jitter(&self) -> u64 {
        self.ttl_with_jitter(self.tiles.cache_ttl.redis_current_ttl_seconds)
    }

    #[must_use]
    pub fn redis_webp_ttl_with_jitter(&self) -> u64 {
        self.ttl_with_jitter(self.tiles.cache_ttl.redis_webp_ttl_seconds)
    }

    #[must_use]
    pub fn redis_rgba_ttl_with_jitter(&self) -> u64 {
        self.ttl_with_jitter(self.tiles.cache_ttl.redis_rgba_ttl_seconds)
    }

    #[must_use]
    pub fn redis_missing_sentinel_ttl_with_jitter(&self) -> u64 {
        self.ttl_with_jitter(self.tiles.cache_ttl.redis_missing_sentinel_ttl_seconds)
    }
}

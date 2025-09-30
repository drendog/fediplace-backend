use deadpool_redis::Pool as RedisPool;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::{Arc, atomic::AtomicUsize};
use tokio::sync::broadcast;

use domain::credits::CreditConfig;
use domain::{events::TileVersionEvent, tile::PaletteBufferPool};
use fedi_wplace_adapters::shared::app_state::AppState as AdaptersAppState;
use fedi_wplace_adapters::{
    incoming::{
        http_axum::middleware::rate_limit::{RateLimiter, create_websocket_rate_limiter},
        ws_axum::WsAdapterPolicy,
    },
    outgoing::{
        email_sender::{
            console_email_sender::ConsoleEmailSender,
            smtp_email_sender::{SmtpEmailConfig, SmtpEmailSender},
        },
        events_broadcast::tokio_broadcast::TokioBroadcastEventsAdapter,
        image_rs::webp_codec_image::{ImageWebpAdapter, ImageWebpConfig},
        passwords::argon2::Argon2PasswordHasher,
        postgres_sqlx::{
            ban_store_postgres::PostgresBanStoreAdapter,
            credit_store_postgres::PostgresCreditStoreAdapter,
            pixel_history_store_postgres::PostgresPixelHistoryStoreAdapter,
            user_store_postgres::PostgresUserStoreAdapter,
        },
        redis_deadpool::{
            subscription_redis::RedisSubscriptionAdapter, tile_cache_redis::RedisTileCacheAdapter,
        },
        tokio_spawn::{TokioTaskSpawnAdapter, webp_timeout_tokio::TokioWebPTimeoutAdapter},
    },
};
use fedi_wplace_application::error::AppError;
use fedi_wplace_application::infrastructure_config::{Config, EmailBackend};
use fedi_wplace_application::ports::incoming::tiles::{
    MetricsQueryUseCase, PaintPixelsUseCase, PixelHistoryQueryUseCase, PixelInfoQueryUseCase,
    TilesQueryUseCase,
};
use fedi_wplace_application::ports::outgoing::{
    ban_store::BanStorePort, credit_store::CreditStorePort, email_sender::EmailSenderPort,
    events::EventsPort, image_codec::ImageCodecPort, password_hasher::PasswordHasherPort,
    pixel_history_store::PixelHistoryStorePort, subscription_port::SubscriptionPort,
    tile_cache::TileCachePort, user_store::UserStorePort,
};
use fedi_wplace_application::{
    admin::service::AdminService,
    auth::service::AuthService,
    ban::service::BanService,
    config::TileSettings,
    ports::incoming::{
        admin::AdminUseCase, auth::AuthUseCase, ban::BanUseCase, subscriptions::SubscriptionUseCase,
    },
    subscriptions::service::SubscriptionService,
    tiles::service::PaletteColorLookup,
    tiles::service::{TileService, TileServiceDeps},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    db_pool: PgPool,
    redis_pool: RedisPool,
    pub tile_service: Arc<TileService>,
    pub subscription_service: Arc<dyn SubscriptionUseCase>,
    pub auth_service: Arc<dyn AuthUseCase>,
    pub admin_service: Arc<dyn AdminUseCase>,
    pub ban_service: Arc<dyn BanUseCase>,
    pub ws_broadcast: broadcast::Sender<TileVersionEvent>,
    pub websocket_rate_limiter: Option<Arc<RateLimiter>>,
    pub active_websocket_connections: Arc<AtomicUsize>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, AppError> {
        let config = Arc::new(config);

        let (palette_color_lookup, palette_buffer_pool) = Self::create_palette_components(&config);
        let (db_pool, redis_pool) = Self::create_database_connections(&config).await?;
        let (ws_broadcast, _) = broadcast::channel(config.websocket.broadcast_buffer_size);

        let tile_service = Self::create_tile_service(
            &config,
            &palette_color_lookup,
            &palette_buffer_pool,
            &db_pool,
            &redis_pool,
            &ws_broadcast,
        )?;

        let subscription_service = Self::create_subscription_service(&config, &redis_pool);
        let auth_service = Self::create_auth_service(&config, &db_pool)?;
        let admin_service = Self::create_admin_service(&config, &db_pool);
        let ban_service = Self::create_ban_service(&config, &db_pool);

        let websocket_rate_limiter = if config.rate_limit.enabled {
            Some(create_websocket_rate_limiter(&config.rate_limit))
        } else {
            None
        };

        Ok(Self {
            config,
            db_pool,
            redis_pool,
            tile_service,
            subscription_service,
            auth_service,
            admin_service,
            ban_service,
            ws_broadcast,
            websocket_rate_limiter,
            active_websocket_connections: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn create_palette_components(
        config: &Config,
    ) -> (Arc<PaletteColorLookup>, Arc<PaletteBufferPool>) {
        let palette_color_lookup = Arc::new(PaletteColorLookup::from_color_palette(
            &config.color_palette.colors,
        ));
        let palette_buffer_pool = Arc::new(PaletteBufferPool::new(
            config.tiles.tile_size,
            config.tiles.buffer_pool_max_size,
        ));
        (palette_color_lookup, palette_buffer_pool)
    }

    async fn create_database_connections(config: &Config) -> Result<(PgPool, RedisPool), AppError> {
        let db_pool = PgPoolOptions::new()
            .max_connections(config.db.pool_size)
            .connect(config.db.database_url())
            .await
            .map_err(|e| AppError::DatabaseError {
                message: format!("Failed to connect to database: {}", e),
            })?;

        let redis_config = deadpool_redis::Config::from_url(&config.redis.redis_url);
        let redis_pool = redis_config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| AppError::CacheError {
                message: format!("Failed to create Redis pool: {}", e),
            })?;

        Ok((db_pool, redis_pool))
    }

    fn create_tile_service(
        config: &Config,
        palette_color_lookup: &Arc<PaletteColorLookup>,
        palette_buffer_pool: &Arc<PaletteBufferPool>,
        db_pool: &PgPool,
        redis_pool: &RedisPool,
        ws_broadcast: &broadcast::Sender<TileVersionEvent>,
    ) -> Result<Arc<TileService>, AppError> {
        let webp_config = ImageWebpConfig {
            tile_size: config.tiles.tile_size,
        };

        let cache_port: Arc<dyn TileCachePort> = Arc::new(RedisTileCacheAdapter::new(
            redis_pool.clone(),
            &config.environment.env,
            config.redis_current_ttl_with_jitter(),
            config.redis_webp_ttl_with_jitter(),
            config.redis_rgba_ttl_with_jitter(),
            config.redis_missing_sentinel_ttl_with_jitter(),
        ));
        let pixel_history_store: Arc<dyn PixelHistoryStorePort> =
            Arc::new(PostgresPixelHistoryStoreAdapter::new(
                db_pool.clone(),
                config.tiles.tile_size,
                config.db.query_timeout_secs,
            ));
        let credit_store: Arc<dyn CreditStorePort> = Arc::new(PostgresCreditStoreAdapter::new(
            db_pool.clone(),
            config.db.query_timeout_secs,
        ));
        let codec_port: Arc<dyn ImageCodecPort> = Arc::new(ImageWebpAdapter::new(webp_config));
        let events_port: Arc<dyn EventsPort> =
            Arc::new(TokioBroadcastEventsAdapter::new(ws_broadcast.clone()));

        let tile_settings = Arc::new(TileSettings {
            tile_size: config.tiles.tile_size,
            pixel_size: config.tiles.pixel_size,
            palette: config.color_palette.colors.clone().into(),
            transparency_color_id: config
                .color_palette
                .get_transparency_color_id()
                .unwrap_or(255),
            color_palette_config: Arc::new(config.color_palette.clone()),
        });

        let tile_service = TileService::new(
            &tile_settings,
            TileServiceDeps {
                cache_port,
                codec_port: Arc::clone(&codec_port),
                webp_timeout_port: Arc::new(TokioWebPTimeoutAdapter::new(Arc::clone(&codec_port))),
                palette_buffer_pool: Arc::clone(palette_buffer_pool),
                palette_color_lookup: Arc::clone(palette_color_lookup),
                events_port,
                task_spawn_port: Arc::new(TokioTaskSpawnAdapter::new()),
                pixel_history_store,
                credit_store,
                credit_config: CreditConfig::new(
                    config.credits.max_charges,
                    config.credits.charge_cooldown_seconds,
                ),
            },
        )?;

        Ok(tile_service)
    }

    fn create_subscription_service(
        config: &Config,
        redis_pool: &RedisPool,
    ) -> Arc<dyn SubscriptionUseCase> {
        let subscription_port: Arc<dyn SubscriptionPort> = Arc::new(RedisSubscriptionAdapter::new(
            redis_pool.clone(),
            config.ws_policy.max_tiles_per_ip,
            config.ws_policy.subscription_ttl_secs * 1000,
        ));
        Arc::new(SubscriptionService::new(subscription_port))
    }

    fn create_auth_service(
        config: &Config,
        db_pool: &PgPool,
    ) -> Result<Arc<dyn AuthUseCase>, AppError> {
        let user_store_port: Arc<dyn UserStorePort> = Arc::new(PostgresUserStoreAdapter::new(
            db_pool.clone(),
            config.db.query_timeout_secs,
        ));
        let password_hasher_port: Arc<dyn PasswordHasherPort> = Arc::new(
            Argon2PasswordHasher::from_config_or_default(&config.auth.argon2),
        );

        let email_sender_port: Arc<dyn EmailSenderPort> = match config.auth.email.email_backend {
            EmailBackend::Console => {
                Arc::new(ConsoleEmailSender::new(config.auth.public_base_url.clone()))
            }
            EmailBackend::Smtp => {
                let smtp_config = &config.auth.email.smtp;
                let email_config = SmtpEmailConfig {
                    smtp_host: smtp_config.host.clone(),
                    smtp_port: smtp_config.port,
                    username: smtp_config.username.clone(),
                    password: smtp_config.password.clone(),
                    from_email: smtp_config.from_email.clone(),
                    from_name: smtp_config.from_name.clone(),
                    base_url: config.auth.public_base_url.clone(),
                    use_tls: smtp_config.use_tls,
                };
                let smtp_sender = SmtpEmailSender::new(email_config)?;
                Arc::new(smtp_sender)
            }
        };

        Ok(Arc::new(AuthService::new(
            user_store_port,
            password_hasher_port,
            email_sender_port,
        )))
    }

    fn create_admin_service(config: &Config, db_pool: &PgPool) -> Arc<dyn AdminUseCase> {
        let user_store_port: Arc<dyn UserStorePort> = Arc::new(PostgresUserStoreAdapter::new(
            db_pool.clone(),
            config.db.query_timeout_secs,
        ));
        Arc::new(AdminService::new(user_store_port))
    }

    fn create_ban_service(config: &Config, db_pool: &PgPool) -> Arc<dyn BanUseCase> {
        let ban_store_port: Arc<dyn BanStorePort> = Arc::new(PostgresBanStoreAdapter::new(
            db_pool.clone(),
            config.db.query_timeout_secs,
        ));
        let user_store_port: Arc<dyn UserStorePort> = Arc::new(PostgresUserStoreAdapter::new(
            db_pool.clone(),
            config.db.query_timeout_secs,
        ));
        Arc::new(BanService::new(ban_store_port, user_store_port))
    }

    pub fn db_pool(&self) -> &PgPool {
        &self.db_pool
    }

    pub fn redis_pool(&self) -> &RedisPool {
        &self.redis_pool
    }

    #[allow(clippy::type_complexity)]
    pub fn to_adapters_state(
        self,
    ) -> (
        AdaptersAppState,
        Arc<dyn UserStorePort>,
        Arc<dyn PasswordHasherPort>,
        Arc<dyn BanStorePort>,
    ) {
        let ws_policy = WsAdapterPolicy {
            heartbeat_refresh_secs: self.config.ws_policy.heartbeat_refresh_secs,
            max_tiles_per_ip: self.config.ws_policy.max_tiles_per_ip,
            subscription_ttl_secs: self.config.ws_policy.subscription_ttl_secs,
        };

        let user_store_port: Arc<dyn UserStorePort> = Arc::new(PostgresUserStoreAdapter::new(
            self.db_pool.clone(),
            self.config.db.query_timeout_secs,
        ));
        let password_hasher_port: Arc<dyn PasswordHasherPort> = Arc::new(
            Argon2PasswordHasher::from_config_or_default(&self.config.auth.argon2),
        );
        let ban_store_port: Arc<dyn BanStorePort> = Arc::new(PostgresBanStoreAdapter::new(
            self.db_pool.clone(),
            self.config.db.query_timeout_secs,
        ));
        let admin_service = Arc::new(AdminService::new(Arc::clone(&user_store_port)));

        let adapters_state = AdaptersAppState::new(
            self.config,
            ws_policy,
            Arc::clone(&self.tile_service) as Arc<dyn TilesQueryUseCase + Send + Sync>,
            Arc::clone(&self.tile_service) as Arc<dyn PaintPixelsUseCase + Send + Sync>,
            Arc::clone(&self.tile_service) as Arc<dyn MetricsQueryUseCase + Send + Sync>,
            Arc::clone(&self.tile_service) as Arc<dyn PixelHistoryQueryUseCase + Send + Sync>,
            Arc::clone(&self.tile_service) as Arc<dyn PixelInfoQueryUseCase + Send + Sync>,
            self.subscription_service,
            self.auth_service,
            admin_service,
            self.ban_service,
            self.ws_broadcast,
            self.websocket_rate_limiter,
            self.active_websocket_connections,
        );

        (
            adapters_state,
            user_store_port,
            password_hasher_port,
            ban_store_port,
        )
    }
}

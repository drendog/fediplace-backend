use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::broadcast;

use fedi_wplace_application::infrastructure_config::Config;

use crate::incoming::http_axum::middleware::rate_limit::RateLimiter;
use crate::incoming::ws_axum::WsAdapterPolicy;

use domain::events::TileVersionEvent;
use fedi_wplace_application::{
    ports::{
        incoming::{
            admin::AdminUseCase,
            auth::AuthUseCase,
            ban::BanUseCase,
            subscriptions::SubscriptionUseCase,
            tiles::{
                MetricsQueryUseCase, PaintPixelsUseCase, PixelHistoryQueryUseCase,
                PixelInfoQueryUseCase, TilesQueryUseCase,
            },
        },
        outgoing::credit_store::DynCreditStorePort,
    },
    world::service::WorldService,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub ws_policy: WsAdapterPolicy,
    pub tiles_query_service: Arc<dyn TilesQueryUseCase + Send + Sync>,
    pub paint_pixels_service: Arc<dyn PaintPixelsUseCase + Send + Sync>,
    pub metrics_query_service: Arc<dyn MetricsQueryUseCase + Send + Sync>,
    pub pixel_history_query_service: Arc<dyn PixelHistoryQueryUseCase + Send + Sync>,
    pub pixel_info_query_service: Arc<dyn PixelInfoQueryUseCase + Send + Sync>,
    pub subscription_service: Arc<dyn SubscriptionUseCase + Send + Sync>,
    pub auth_use_case: Arc<dyn AuthUseCase + Send + Sync>,
    pub admin_use_case: Arc<dyn AdminUseCase + Send + Sync>,
    pub ban_use_case: Arc<dyn BanUseCase + Send + Sync>,
    pub world_service: Arc<WorldService>,
    pub credit_store: DynCreditStorePort,
    pub ws_broadcast: broadcast::Sender<TileVersionEvent>,
    pub websocket_rate_limiter: Option<Arc<RateLimiter>>,
    pub active_websocket_connections: Arc<AtomicUsize>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Arc<Config>,
        ws_policy: WsAdapterPolicy,
        tiles_query_service: Arc<dyn TilesQueryUseCase + Send + Sync>,
        paint_pixels_service: Arc<dyn PaintPixelsUseCase + Send + Sync>,
        metrics_query_service: Arc<dyn MetricsQueryUseCase + Send + Sync>,
        pixel_history_query_service: Arc<dyn PixelHistoryQueryUseCase + Send + Sync>,
        pixel_info_query_service: Arc<dyn PixelInfoQueryUseCase + Send + Sync>,
        subscription_service: Arc<dyn SubscriptionUseCase + Send + Sync>,
        auth_use_case: Arc<dyn AuthUseCase + Send + Sync>,
        admin_use_case: Arc<dyn AdminUseCase + Send + Sync>,
        ban_use_case: Arc<dyn BanUseCase + Send + Sync>,
        world_service: Arc<WorldService>,
        credit_store: DynCreditStorePort,
        ws_broadcast: broadcast::Sender<TileVersionEvent>,
        websocket_rate_limiter: Option<Arc<RateLimiter>>,
        active_websocket_connections: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            config,
            ws_policy,
            tiles_query_service,
            paint_pixels_service,
            metrics_query_service,
            pixel_history_query_service,
            pixel_info_query_service,
            subscription_service,
            auth_use_case,
            admin_use_case,
            ban_use_case,
            world_service,
            credit_store,
            ws_broadcast,
            websocket_rate_limiter,
            active_websocket_connections,
        }
    }

    pub fn increment_websocket_connections(&self) -> usize {
        self.active_websocket_connections
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }

    pub fn decrement_websocket_connections(&self) -> usize {
        self.active_websocket_connections
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1)
    }

    pub fn get_websocket_connection_count(&self) -> usize {
        self.active_websocket_connections.load(Ordering::Relaxed)
    }

    pub fn check_websocket_connection_limit(&self) -> bool {
        if let Some(max_connections) = self.config.websocket.max_connections {
            self.get_websocket_connection_count() < max_connections
        } else {
            true
        }
    }
}

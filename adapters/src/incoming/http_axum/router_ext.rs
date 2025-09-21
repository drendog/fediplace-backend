use axum::{Router, middleware};
use axum_login::AuthManagerLayer;
use std::sync::Arc;
use tower_sessions_redis_store::{RedisStore, fred::prelude::Client};

use crate::incoming::http_axum::{
    auth::backend::AuthBackend,
    middleware::{
        rate_limit::{RateLimiter, rate_limit_middleware},
        request_id::request_id_middleware,
    },
};

pub trait RouterExt<State> {
    fn with_request_id(self) -> Self;
    fn with_auth(self, layer: AuthManagerLayer<AuthBackend, RedisStore<Client>>) -> Self;
    fn with_rate_limit(self, limiter: Arc<RateLimiter>) -> Self;
}

impl<State> RouterExt<State> for Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    fn with_request_id(self) -> Self {
        self.layer(middleware::from_fn(request_id_middleware))
    }

    fn with_auth(self, layer: AuthManagerLayer<AuthBackend, RedisStore<Client>>) -> Self {
        self.layer(layer)
    }

    fn with_rate_limit(self, limiter: Arc<RateLimiter>) -> Self {
        self.layer(middleware::from_fn(move |conn_info, req, next| {
            let limiter_clone = Arc::clone(&limiter);
            rate_limit_middleware(limiter_clone, conn_info, req, next)
        }))
    }
}

use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method},
    middleware,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::bootstrap::state::AppState;
use fedi_wplace_adapters::shared::app_state::AppState as AdaptersAppState;
use fedi_wplace_application::error::AppError;

use fedi_wplace_adapters::incoming::http_axum::{
    middleware::rate_limit::{create_general_rate_limiter, rate_limit_middleware},
    routes::build_application_router,
};

pub async fn create_router(state: AppState) -> Result<Router, AppError> {
    let (adapters_state, user_store, password_hasher, ban_store) = state.to_adapters_state();
    let cors_layer = create_cors_layer(&adapters_state);

    let application_router =
        build_application_router(&adapters_state, user_store, password_hasher, ban_store).await?;

    let router_with_rate_limiting = if adapters_state.config.rate_limit.enabled {
        let global_rate_limiter = create_general_rate_limiter(&adapters_state.config.rate_limit);
        application_router.layer(middleware::from_fn(move |conn_info, req, next| {
            let limiter = Arc::clone(&global_rate_limiter);
            rate_limit_middleware(limiter, conn_info, req, next)
        }))
    } else {
        application_router
    };

    Ok(router_with_rate_limiting
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors_layer),
        )
        .with_state(adapters_state))
}

fn create_cors_layer(state: &AdaptersAppState) -> CorsLayer {
    let base_cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
            HeaderName::from_static("accept"),
            HeaderName::from_static("origin"),
            HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true);

    match &state.config.server.cors_origin {
        Some(origin) => base_cors.allow_origin(
            origin
                .parse::<HeaderValue>()
                .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:3000")),
        ),
        None => base_cors.allow_origin(HeaderValue::from_static("http://localhost:3000")),
    }
}

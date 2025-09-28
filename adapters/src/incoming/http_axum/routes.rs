use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};
use axum_login::{AuthManagerLayer, AuthManagerLayerBuilder};
use fedi_wplace_application::error::AppError;
use tower_sessions_redis_store::{RedisStore, fred::prelude::Client};
#[cfg(feature = "docs")]
use utoipa::OpenApi;
#[cfg(feature = "docs")]
use utoipa_swagger_ui::SwaggerUi;

use crate::shared::app_state::AppState;
use crate::{
    incoming::http_axum::{
        auth::{
            backend::AuthBackend,
            oauth_google::{google_auth_callback, google_auth_start},
            session::{SessionConfig, create_session_layer},
        },
        handlers::{
            admin::assign_role_to_user,
            auth::{
                login_handler, logout_handler, me_handler, register_handler,
                update_username_handler, verify_email_handler,
            },
            ban::{ban_user, get_user_ban_status, list_active_bans, unban_user},
            health::health_check,
            palette::get_palette,
            pixel_info::get_pixel_info,
            tiles::{paint_pixels_batch, serve_tile, serve_tile_head},
        },
        middleware::{
            admin_auth::require_admin_role,
            rate_limit::{
                create_auth_rate_limiter, create_paint_rate_limiter, create_tile_rate_limiter,
            },
            verification::require_email_verification,
        },
        router_ext::RouterExt,
    },
    incoming::ws_axum::endpoint::websocket_handler,
};
use fedi_wplace_application::ports::outgoing::{
    ban_store::DynBanStorePort, password_hasher::DynPasswordHasherPort,
    user_store::DynUserStorePort,
};

#[cfg(feature = "docs")]
use crate::incoming::http_axum::docs::ApiDoc;

pub async fn build_application_router(
    state: &AppState,
    user_store: DynUserStorePort,
    password_hasher: DynPasswordHasherPort,
    ban_store: DynBanStorePort,
) -> Result<Router<AppState>, AppError> {
    let core_routes = build_core_routes();
    let (auth_routes, auth_layer) =
        build_auth_routes(state, user_store, password_hasher, ban_store).await?;
    let tile_routes = build_tile_routes_with_auth(state, auth_layer.clone());
    let admin_routes = build_admin_routes_with_auth(auth_layer);

    Ok(core_routes
        .merge(tile_routes)
        .merge(auth_routes)
        .merge(admin_routes))
}

fn build_core_routes() -> Router<AppState> {
    let router = Router::new()
        .route("/palette", get(get_palette))
        .route("/pixel/{x}/{y}", get(get_pixel_info))
        .route("/live", get(websocket_handler));

    #[cfg(feature = "docs")]
    {
        router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
    }

    #[cfg(not(feature = "docs"))]
    {
        router
    }
}

fn build_tile_routes_with_auth(
    state: &AppState,
    auth_layer: AuthManagerLayer<AuthBackend, RedisStore<Client>>,
) -> Router<AppState> {
    let tile_routes = Router::new().route("/tiles/{x}/{y}", get(serve_tile).head(serve_tile_head));
    let paint_routes = Router::new().route("/tiles/{x}/{y}/pixels", post(paint_pixels_batch));

    let tile_routes_final = if state.config.rate_limit.enabled {
        let tile_limiter = create_tile_rate_limiter(&state.config.rate_limit);
        tile_routes.with_rate_limit(tile_limiter)
    } else {
        tile_routes
    };

    let paint_routes_final = if state.config.rate_limit.enabled {
        let paint_limiter = create_paint_rate_limiter(&state.config.rate_limit);
        paint_routes
            .layer(middleware::from_fn(require_email_verification))
            .with_auth(auth_layer)
            .with_rate_limit(paint_limiter)
    } else {
        paint_routes
            .layer(middleware::from_fn(require_email_verification))
            .with_auth(auth_layer)
    };

    tile_routes_final.merge(paint_routes_final)
}

fn build_admin_routes_with_auth(
    auth_layer: AuthManagerLayer<AuthBackend, RedisStore<Client>>,
) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/users/{user_id}/roles/{role_id}", put(assign_role_to_user))
        .route("/users/{user_id}/ban", post(ban_user))
        .route("/users/{user_id}/ban", delete(unban_user))
        .route("/users/{user_id}/ban", get(get_user_ban_status))
        .route("/bans", get(list_active_bans))
        .layer(middleware::from_fn(require_admin_role))
        .with_auth(auth_layer)
}

async fn build_auth_routes(
    state: &AppState,
    user_store: DynUserStorePort,
    password_hasher: DynPasswordHasherPort,
    ban_store: DynBanStorePort,
) -> Result<
    (
        Router<AppState>,
        AuthManagerLayer<AuthBackend, RedisStore<Client>>,
    ),
    AppError,
> {
    let same_site_policy = if state.config.auth.cookie_secure {
        "None".to_string()
    } else {
        "Lax".to_string()
    };

    let session_config = SessionConfig {
        cookie_name: state.config.auth.cookie_name.clone(),
        secure: state.config.auth.cookie_secure,
        same_site: same_site_policy,
    };
    let session_layer =
        create_session_layer(&state.config.redis.redis_url, &session_config).await?;

    let auth_backend = AuthBackend::new(user_store, password_hasher, ban_store);

    let rate_limited_routes = Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler));

    let other_routes = Router::new()
        .route("/auth/logout", post(logout_handler))
        .route("/auth/me", get(me_handler))
        .route("/auth/username", put(update_username_handler))
        .route("/auth/verify", get(verify_email_handler))
        .route("/auth/google/start", get(google_auth_start))
        .route("/auth/google/callback", get(google_auth_callback));

    let final_routes = if state.config.rate_limit.enabled {
        let auth_limiter = create_auth_rate_limiter(&state.config.rate_limit);
        rate_limited_routes
            .with_rate_limit(auth_limiter)
            .merge(other_routes)
    } else {
        rate_limited_routes.merge(other_routes)
    };

    let auth_manager_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();
    let routes_with_session = final_routes.layer(auth_manager_layer.clone());

    let routes_with_request_id = routes_with_session.with_request_id();

    Ok((routes_with_request_id, auth_manager_layer))
}

#[cfg(any(
    feature = "adapters",
    feature = "axum",
    feature = "sqlx",
    feature = "deadpool-redis",
    feature = "image"
))]
compile_error!("application must not depend on adapters/framework crates");

pub mod auth;
pub mod config;
pub mod contracts;
pub mod error;
pub mod infrastructure_config;
pub mod ports;
pub mod subscriptions;
pub mod tiles;

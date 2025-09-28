#[cfg(any(
    feature = "adapters",
    feature = "axum",
    feature = "sqlx",
    feature = "deadpool-redis",
    feature = "image"
))]
compile_error!("application must not depend on adapters/framework crates");

pub mod admin;
pub mod auth;
pub mod ban;
pub mod config;
pub mod contracts;
pub mod error;
pub mod infrastructure_config;
pub mod ports;
pub mod subscriptions;
pub mod tiles;

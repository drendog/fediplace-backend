#[cfg(feature = "docs")]
pub mod docs;

pub(crate) mod core;
pub(crate) mod error_mapper;
pub(crate) mod router_ext;

// keep public for OpenAPI docs
pub mod auth;
pub mod dto;
pub mod handlers;
pub mod middleware;
pub mod routes;

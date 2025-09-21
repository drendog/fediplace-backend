pub(crate) mod buffer;
pub(crate) mod connection;
pub(crate) mod handler;
pub(crate) mod ip_utils;
pub(crate) mod subscriptions;

pub mod endpoint; // Keep public for router access
pub mod protocol; // Keep public for external API access to types

#[derive(Debug, Clone)]
pub struct WsAdapterPolicy {
    pub heartbeat_refresh_secs: u64,
    pub max_tiles_per_ip: usize,
    pub subscription_ttl_secs: u64,
}

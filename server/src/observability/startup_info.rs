use fedi_wplace_application::infrastructure_config::{
    Config, RateLimitConfig, TileConfig, WsPolicyConfig,
};
use tracing::info;

pub fn print_api_info(config: &Config) {
    print_api_documentation_info(config);
    print_configuration_info(config);
    print_rate_limiting_info(config);
}

fn print_api_documentation_info(config: &Config) {
    let base_url = format!("http://{}", config.server_address());
    info!("📋 API Documentation:");
    info!("  📖 Swagger UI: {}/docs", base_url);
    info!("  📄 OpenAPI JSON: {}/api-docs/openapi.json", base_url);
}

fn print_configuration_info(config: &Config) {
    info!("⚙️  Configuration:");
    print_tile_configuration(&config.tiles);
    print_database_configuration();
    print_cache_configuration(config);
    print_websocket_configuration(&config.ws_policy);
}

fn print_tile_configuration(tiles_config: &TileConfig) {
    info!(
        "  📐 Tile size: {}x{} pixels",
        tiles_config.tile_size, tiles_config.tile_size
    );
    info!(
        "  🎨 Pixel size: {}x{} pixels",
        tiles_config.pixel_size, tiles_config.pixel_size
    );
}

fn print_database_configuration() {
    info!("  🗄️  Database: PostgreSQL with connection pooling");
}

fn print_cache_configuration(config: &Config) {
    info!(
        "  📦 Cache: Redis current/webp/rgba ({}s/{}s/{}s), Database (PostgreSQL)",
        config.tiles.cache_ttl.redis_current_ttl_seconds,
        config.tiles.cache_ttl.redis_webp_ttl_seconds,
        config.tiles.cache_ttl.redis_rgba_ttl_seconds
    );
}

fn print_websocket_configuration(ws_policy: &WsPolicyConfig) {
    info!(
        "  📋 WebSocket Policy: max {} tiles/IP, TTL {}s, heartbeat {}s",
        ws_policy.max_tiles_per_ip,
        ws_policy.subscription_ttl_secs,
        ws_policy.heartbeat_refresh_secs
    );
}

fn print_rate_limiting_info(config: &Config) {
    if config.rate_limit.enabled {
        info!("  🚦 Rate Limiting: ENABLED");
        print_rate_limits(&config.rate_limit);
    } else {
        info!("  🚦 Rate Limiting: DISABLED");
    }
}

#[allow(clippy::cognitive_complexity)]
fn print_rate_limits(rate_limit: &RateLimitConfig) {
    info!(
        "    • Paint: {}/min per IP (burst: {})",
        rate_limit.paint_requests_per_minute,
        rate_limit.paint_requests_per_minute * rate_limit.burst_size_multiplier
    );
    info!(
        "    • Tiles: {}/min per IP (burst: {})",
        rate_limit.tile_requests_per_minute,
        rate_limit.tile_requests_per_minute * rate_limit.burst_size_multiplier
    );
    info!(
        "    • Global: {}/min per IP (burst: {})",
        rate_limit.global_requests_per_minute,
        rate_limit.global_requests_per_minute * rate_limit.burst_size_multiplier
    );
    info!(
        "    • WebSocket: {}/min per IP (burst: {})",
        rate_limit.websocket_messages_per_minute,
        rate_limit.websocket_messages_per_minute * rate_limit.burst_size_multiplier
    );
}

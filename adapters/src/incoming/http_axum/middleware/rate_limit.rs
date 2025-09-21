use axum::{
    extract::{ConnectInfo, Request},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, RETRY_AFTER},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::{MissedTickBehavior, interval};

use fedi_wplace_application::infrastructure_config::RateLimitConfig;

#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    pub requests: u32,
    pub window_start: Instant,
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: Instant,
    pub retry_after_seconds: Option<u64>,
}

impl RateLimitInfo {
    pub fn to_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(
            "RateLimit-Limit",
            HeaderValue::from_str(&self.limit.to_string()).unwrap_or(HeaderValue::from_static("0")),
        );

        headers.insert(
            "RateLimit-Remaining",
            HeaderValue::from_str(&self.remaining.to_string())
                .unwrap_or(HeaderValue::from_static("0")),
        );

        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        let time_until_reset = self.reset_time.saturating_duration_since(now_instant);
        let reset_system_time = now_system + time_until_reset;

        let reset_timestamp = reset_system_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        headers.insert(
            "RateLimit-Reset",
            HeaderValue::from_str(&reset_timestamp.to_string())
                .unwrap_or(HeaderValue::from_static("0")),
        );

        if let Some(retry_after) = self.retry_after_seconds {
            headers.insert(
                RETRY_AFTER,
                HeaderValue::from_str(&retry_after.to_string())
                    .unwrap_or(HeaderValue::from_static("0")),
            );
        }

        headers
    }
}

#[derive(Debug)]
pub enum RateLimitResult {
    Allowed(RateLimitInfo),
    Denied(RateLimitInfo),
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub store: Arc<DashMap<IpAddr, RateLimitEntry>>,
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32, burst_size_multiplier: u32) -> Self {
        let burst_size = requests_per_minute * burst_size_multiplier;
        let store = Arc::new(DashMap::new());

        let store_clone = Arc::clone(&store);
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(60));
            cleanup_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                cleanup_interval.tick().await;
                let now = Instant::now();
                store_clone.retain(|_, entry: &mut RateLimitEntry| {
                    now.duration_since(entry.window_start) < Duration::from_secs(60)
                });
            }
        });

        Self {
            store,
            requests_per_minute,
            burst_size,
        }
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> RateLimitResult {
        let now = Instant::now();

        let mut entry = self.store.entry(ip).or_insert_with(|| RateLimitEntry {
            requests: 0,
            window_start: now,
        });

        let window_expired = now.duration_since(entry.window_start) >= Duration::from_secs(60);
        if window_expired {
            entry.window_start = now;
            entry.requests = 0;
        }

        let remaining = self.burst_size.saturating_sub(entry.requests);
        let reset_time = entry.window_start + Duration::from_secs(60);

        let within_burst_limit = entry.requests < self.burst_size;
        if within_burst_limit {
            entry.requests += 1;

            let rate_limit_info = RateLimitInfo {
                limit: self.burst_size,
                remaining: remaining.saturating_sub(1), // -1 because we just consumed one
                reset_time,
                retry_after_seconds: None,
            };

            RateLimitResult::Allowed(rate_limit_info)
        } else {
            let retry_after_seconds = reset_time.saturating_duration_since(now).as_secs();

            let rate_limit_info = RateLimitInfo {
                limit: self.burst_size,
                remaining: 0,
                reset_time,
                retry_after_seconds: Some(retry_after_seconds),
            };

            RateLimitResult::Denied(rate_limit_info)
        }
    }
}

fn merge_headers_safe(target: &mut HeaderMap, source: &HeaderMap) {
    for (key, value) in source {
        if !target.contains_key(key) {
            target.insert(key, value.clone());
        }
    }
}

pub async fn rate_limit_middleware(
    rate_limiter: Arc<RateLimiter>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let client_ip = addr.ip();

    match rate_limiter.check_rate_limit(client_ip) {
        RateLimitResult::Allowed(rate_info) => {
            let mut response = next.run(request).await;

            let rate_headers = rate_info.to_headers();
            merge_headers_safe(response.headers_mut(), &rate_headers);

            response
        }
        RateLimitResult::Denied(rate_info) => {
            tracing::warn!(
                "Rate limit exceeded for IP: {} on {} {}",
                client_ip,
                request.method(),
                request.uri()
            );

            let mut headers = rate_info.to_headers();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

            (
                StatusCode::TOO_MANY_REQUESTS,
                headers,
                "Rate limit exceeded",
            )
                .into_response()
        }
    }
}

pub fn create_paint_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(
        config.paint_requests_per_minute,
        config.burst_size_multiplier,
    ))
}

pub fn create_tile_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(
        config.tile_requests_per_minute,
        config.burst_size_multiplier,
    ))
}

pub fn create_general_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(
        config.global_requests_per_minute,
        config.burst_size_multiplier,
    ))
}

pub fn create_websocket_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(
        config.websocket_messages_per_minute,
        config.burst_size_multiplier,
    ))
}

pub fn create_auth_rate_limiter(config: &RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(
        config.auth_requests_per_minute,
        config.burst_size_multiplier,
    ))
}

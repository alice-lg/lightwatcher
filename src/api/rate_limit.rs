use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use chrono::{DateTime, Utc};
use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Extension,
};
use tokio::sync::Mutex;

use crate::config::RateLimitConfig;

#[derive(Clone)]
struct RateLimitBucket {
    count: u64,
    window_start: DateTime<Utc>,
}

impl RateLimitBucket {
    fn reset(&mut self) {
        self.count = 0;
        self.window_start = Utc::now();
    }
}

impl Default for RateLimitBucket {
    fn default() -> Self {
        Self{
            count: 0,
            window_start: Utc::now(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Arc<Mutex<HashMap<String, RateLimitBucket>>>,
}

impl RateLimiter {
    /// New rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the request runs into a limit. 
    async fn check_rate_limit(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets.entry(key.into()).or_default();

        if Utc::now().signed_duration_since(bucket.window_start) > self.config.window {
            bucket.reset();
            true
        } else if bucket.count < self.config.requests {
            bucket.count += 1;
            true
        } else {
            false
        }
    }

}

pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(limiter): Extension<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Use IP address as key...
    let key = addr.ip().to_string();
    if limiter.check_rate_limit(&key).await {
        Ok(next.run(request).await)
    } else {
        tracing::warn!(client = key, "rate limit reached");
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

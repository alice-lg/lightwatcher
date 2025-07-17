use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use chrono::{DateTime, Utc};
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
        Self {
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

        if Utc::now().signed_duration_since(bucket.window_start)
            > self.config.window
        {
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
    // Get client identifier: prefer Forwarded header, fallback to IP address
    let mut key = addr.to_string();
    let headers = request.headers();

    // Use header for client identification
    if let Some(hdr) = headers.get(header::FORWARDED) {
        if let Ok(hdr) = hdr.to_str() {
            key = hdr.to_string()
        }
    }

    if limiter.check_rate_limit(&key).await {
        Ok(next.run(request).await)
    } else {
        tracing::warn!(client = key, "rate limit reached");
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_check_rate_limit_blocks_after_limit() {
        let config = RateLimitConfig {
            requests: 2,
            window: Duration::minutes(1),
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("test_key").await);
        assert!(limiter.check_rate_limit("test_key").await);
        assert!(!limiter.check_rate_limit("test_key").await);
    }

    #[tokio::test]
    async fn test_check_rate_limit_different_keys() {
        let config = RateLimitConfig {
            requests: 1,
            window: Duration::minutes(1),
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("key1").await);
        assert!(limiter.check_rate_limit("key2").await);
        assert!(!limiter.check_rate_limit("key1").await);
        assert!(!limiter.check_rate_limit("key2").await);
    }

    #[tokio::test]
    async fn test_check_rate_limit_window_reset() {
        let config = RateLimitConfig {
            requests: 1,
            window: Duration::milliseconds(10),
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("test_key").await);
        assert!(!limiter.check_rate_limit("test_key").await);

        // No idea how else to test this...
        // but I guess this sleep time is sufficiently short.
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;

        assert!(limiter.check_rate_limit("test_key").await);
    }
}

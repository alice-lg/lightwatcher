use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

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
struct RateLimitEntry {
    count: u64,
    window_start: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<Mutex<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the request runs into a limit. 
    async fn is_allowed(&self, key: String) -> bool {
        let mut entries = self.entries.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);

        match entries.get_mut(&key) {
            Some(entry) => {
                if now.duration_since(entry.window_start) > window {
                    // Reset window
                    entry.count = 1;
                    entry.window_start = now;
                    true
                } else if entry.count < self.config.requests {
                    entry.count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                entries.insert(
                    key,
                    RateLimitEntry {
                        count: 1,
                        window_start: now,
                    },
                );
                true
            }
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
    if limiter.is_allowed(key).await {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

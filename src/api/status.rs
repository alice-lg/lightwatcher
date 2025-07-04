use std::sync::Arc;

use anyhow::Result;
use chrono::Duration;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{cache::ResponseCache, responses::StatusResponse, Error},
    bird::Birdc,
    config::CacheConfig,
};

type StatusCache = Arc<Mutex<ResponseCache<StatusResponse>>>;

lazy_static! {
    static ref STATUS_CACHE: StatusCache = {
        let config = CacheConfig {
            max_entries: 1,
            ttl: Duration::new(5, 0).unwrap(),
        };
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
}

/// Get the current status
pub async fn retrieve() -> Result<StatusResponse, Error> {
    let birdc = Birdc::default();

    let res = {
        let cache = STATUS_CACHE.lock().await;
        cache.get("status").cloned()
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let status = birdc.show_status().await?;
            let response = StatusResponse {
                status,
                ..Default::default()
            };
            let mut cache = STATUS_CACHE.lock().await;
            cache.put("status", response.clone());

            Ok(response)
        }
    }
}

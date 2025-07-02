use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{cache::ResponseCache, responses::ProtocolsResponse, Error},
    bird::Birdc,
    config,
};

type ProtocolsCache = Arc<Mutex<ResponseCache<ProtocolsResponse>>>;

lazy_static! {
    static ref BGP_PROTOCOLS_CACHE: ProtocolsCache = {
        let config = config::get_neighbors_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref PROTOCOLS_CACHE: ProtocolsCache = {
        let config = config::get_neighbors_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
}

/// List all protocols (show protocols all)
pub async fn list() -> Result<ProtocolsResponse, Error> {
    let birdc = Birdc::default();

    let res = {
        let cache = PROTOCOLS_CACHE.lock().await;
        match cache.get("all") {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let protocols = birdc.show_protocols().await?;
            let response = ProtocolsResponse {
                protocols,
                ..Default::default()
            };
            let mut cache = PROTOCOLS_CACHE.lock().await;
            cache.put("all", response.clone());
            Ok(response)
        }
    }
}

/// List all neighbors (show protocols all, filter BGP)
pub async fn list_bgp() -> Result<ProtocolsResponse, Error> {
    let birdc = Birdc::default();

    let res = {
        let cache = BGP_PROTOCOLS_CACHE.lock().await;
        match cache.get("all") {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let protocols = birdc.show_protocols_bgp().await?;
            let response = ProtocolsResponse {
                protocols,
                ..Default::default()
            };
            let mut cache = BGP_PROTOCOLS_CACHE.lock().await;
            cache.put("all", response.clone());
            Ok(response)
        }
    }
}

use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{cache::ResponseCache, responses::NeighborsResponse, Error},
    bird::Birdc,
    config,
};

type NeighborsCache = Arc<Mutex<ResponseCache<NeighborsResponse>>>;
type ProtocolsCache = Arc<Mutex<ResponseCache<NeighborsResponse>>>;

lazy_static! {
    static ref NEIGHBORS_CACHE: NeighborsCache = {
        let config = config::get_neighbors_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref PROTOCOLS_CACHE: ProtocolsCache = {
        let config = config::get_neighbors_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
}

/// List all protocols (show protocols all)
pub async fn list() -> Result<NeighborsResponse, Error> {
    let birdc = Birdc::default();

    let res = {
        let cache = NEIGHBORS_CACHE.lock().await;
        match cache.get("all") {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let protocols = birdc.show_protocols_all().await?;
            let response = NeighborsResponse {
                protocols,
                ..Default::default()
            };
            let mut cache = NEIGHBORS_CACHE.lock().await;
            cache.put("all", response.clone());
            Ok(response)
        }
    }
}

/// List all neighbors (show protocols all, filter BGP)
pub async fn list_bgp() -> Result<NeighborsResponse, Error> {
    let birdc = Birdc::default();

    let res = {
        let cache = NEIGHBORS_CACHE.lock().await;
        match cache.get("all") {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let protocols = birdc.show_protocols_bgp_all().await?;
            let response = NeighborsResponse {
                protocols,
                ..Default::default()
            };
            let mut cache = NEIGHBORS_CACHE.lock().await;
            cache.put("all", response.clone());
            Ok(response)
        }
    }
}

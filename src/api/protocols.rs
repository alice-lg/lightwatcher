use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{cache::ResponseCache, responses::ProtocolsResponse, Error},
    bird::{Birdc, ProtocolsMap},
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
        cache.get("all").cloned()
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let mut protocols = birdc.show_protocols_stream().await?;
            let mut mapping = ProtocolsMap::new();
            while let Some(protocol) = protocols.recv().await {
                mapping.insert(protocol.id.clone(), protocol);
            }

            let response = ProtocolsResponse {
                protocols: mapping,
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
        cache.get("all").cloned()
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let mut protocols = birdc.show_protocols_bgp_stream().await?;
            let mut mapping = ProtocolsMap::new();
            while let Some(protocol) = protocols.recv().await {
                mapping.insert(protocol.id.clone(), protocol);
            }

            let response = ProtocolsResponse {
                protocols: mapping,
                ..Default::default()
            };
            let mut cache = BGP_PROTOCOLS_CACHE.lock().await;
            cache.put("all", response.clone());
            Ok(response)
        }
    }
}

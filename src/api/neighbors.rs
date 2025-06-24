use std::sync::Arc;

use anyhow::Result;
use axum::extract::Path;
use chrono::Duration;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{
        cache::ResponseCache,
        responses::{NeighborsResponse, RoutesResponse},
        Error,
    },
    bird::{Birdc, ProtocolID},
    config::CacheConfig,
};

type NeighborsCache = Arc<Mutex<ResponseCache<NeighborsResponse>>>;
type RoutesCache = Arc<Mutex<ResponseCache<RoutesResponse>>>;

lazy_static! {
    static ref NEIGHBORS_CACHE: NeighborsCache = {
        let config = CacheConfig {
            max_entries: 1,
            ttl: Duration::new(300, 0).unwrap(),
        };
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref ROUTES_RECEIVED_CACHE: RoutesCache = {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::new(300, 0).unwrap(),
        };
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref ROUTES_FILTERED_CACHE: RoutesCache = {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::new(300, 0).unwrap(),
        };
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref ROUTES_NO_EXPORT_CACHE: RoutesCache = {
        let config = CacheConfig {
            max_entries: 10,
            ttl: Duration::new(300, 0).unwrap(),
        };
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
}

/// List all neighbors (show protocols all, filter BGP)
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

/// List all routes received for a neighbor
pub async fn list_routes_received(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;

    let res = {
        let cache = ROUTES_RECEIVED_CACHE.lock().await;
        match cache.get(&protocol) {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let routes = birdc.show_route_all_protocol(&protocol).await?;
            let response = RoutesResponse {
                routes,
                ..Default::default()
            };
            let mut cache = ROUTES_RECEIVED_CACHE.lock().await;
            cache.put(&protocol, response.clone());
            Ok(response)
        }
    }
}

/// List all routes filtered by a neighbor
pub async fn list_routes_filtered(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;

    let res = {
        let cache = ROUTES_FILTERED_CACHE.lock().await;
        match cache.get(&protocol) {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let routes =
                birdc.show_route_all_filtered_protocol(&protocol).await?;
            let response = RoutesResponse {
                routes,
                ..Default::default()
            };
            let mut cache = ROUTES_FILTERED_CACHE.lock().await;
            cache.put(&protocol, response.clone());
            Ok(response)
        }
    }
}

/// List all routes not exported
pub async fn list_routes_noexport(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;

    let res = {
        let cache = ROUTES_NO_EXPORT_CACHE.lock().await;
        match cache.get(&protocol) {
            Some(res) => Some(res.clone()),
            None => None,
        }
    };

    match res {
        Some(res) => Ok(res),
        None => {
            let routes =
                birdc.show_route_all_noexport_protocol(&protocol).await?;
            let response = RoutesResponse {
                routes,
                ..Default::default()
            };
            let mut cache = ROUTES_NO_EXPORT_CACHE.lock().await;
            cache.put(&protocol, response.clone());
            Ok(response)
        }
    }
}

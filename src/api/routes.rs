use std::sync::Arc;

use anyhow::Result;
use axum::extract::Path;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::{
    api::{cache::ResponseCache, responses::RoutesResponse, Error},
    bird::{Birdc, ProtocolID, TableID},
    config,
};

type RoutesCache = Arc<Mutex<ResponseCache<RoutesResponse>>>;

lazy_static! {
    static ref ROUTES_RECEIVED_CACHE: RoutesCache = {
        let config = config::get_routes_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref ROUTES_FILTERED_CACHE: RoutesCache = {
        let config = config::get_routes_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
    static ref ROUTES_NO_EXPORT_CACHE: RoutesCache = {
        let config = config::get_routes_cache_config();
        Arc::new(Mutex::new(ResponseCache::new(config)))
    };
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

/// List all routes in a table
pub async fn list_routes_table(
    Path(table): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let table = TableID::parse(&table)?;

    let routes = birdc.show_route_all_table(&table).await?;
    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    Ok(response)
}

/// List all routes in a table
pub async fn list_routes_table_filtered(
    Path(table): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let table = TableID::parse(&table)?;
    let routes = birdc.show_route_all_filtered_table(&table).await?;

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    Ok(response)
}

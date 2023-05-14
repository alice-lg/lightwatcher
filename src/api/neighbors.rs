use std::collections::HashMap;
use std::io::BufReader;

use anyhow::Result;
use axum::extract::Path;
use tokio::task;

use crate::{
    api::{
        responses::{NeighborsResponse, RoutesResponse},
        Error,
    },
    bird,
    parsers::{
        neighbors::NeighborReader, parser::BlockIterator, routes::RE_ROUTES_START,
        routes_worker::RoutesWorkerPool,
    },
    state::{Neighbor, Route},
};

/// List all neighbors (show protocols all, filter BGP)
pub async fn list() -> Result<String, Error> {
    let result = bird::show_protocols_all()?;
    let buf = BufReader::new(result);
    let reader = NeighborReader::new(buf);
    let neighbors: Vec<Neighbor> = reader.filter(|n| !n.id.is_empty()).collect();

    let neighbors: HashMap<String, Neighbor> =
        neighbors.into_iter().map(|n| (n.id.clone(), n)).collect();

    let response = NeighborsResponse {
        protocols: neighbors,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

/// List all routes received for a neighbor
pub async fn list_routes_received(Path(id): Path<String>) -> Result<String, Error> {
    let result = bird::show_route_all_protocol(&id)?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(4);
    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    })
    .await?;
    for result in results_rx {
        let result = result.unwrap();
        routes.extend(result);
    }

    let response = RoutesResponse {
        routes: routes,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

/// List all routes filtered by a neighbor
pub async fn list_routes_filtered(Path(id): Path<String>) -> Result<String, Error> {
    let result = bird::show_route_all_protocol_filtered(&id)?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(4);
    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    })
    .await?;
    for result in results_rx {
        let result = result.unwrap();
        routes.extend(result);
    }

    let response = RoutesResponse {
        routes: routes,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

/// List all routes not exported
pub async fn list_routes_noexport(Path(id): Path<String>) -> Result<String, Error> {
    let result = bird::show_route_all_protocol_noexport(&id)?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(4);
    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    })
    .await?;
    for result in results_rx {
        let result = result.unwrap();
        routes.extend(result);
    }
    let response = RoutesResponse {
        routes: routes,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

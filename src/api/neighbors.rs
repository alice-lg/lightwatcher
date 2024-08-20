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
        neighbors::NeighborReader, parser::BlockIterator,
        routes::RE_ROUTES_START, routes_worker::RoutesWorkerPool,
    },
    state::{Neighbor, Route},
};

/// List all neighbors (show protocols all, filter BGP)
pub async fn list() -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowProtocolsAll)?;
    let buf = BufReader::new(result);
    let reader = NeighborReader::new(buf);
    let neighbors: Vec<Neighbor> =
        reader.filter(|n| !n.id.is_empty()).collect();

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
pub async fn list_routes_received(
    Path(id): Path<String>,
) -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowRouteAllProtocol(id))?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, mut results_rx) = RoutesWorkerPool::spawn();

    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    });

    while let Some(result) = results_rx.recv().await {
        let result = result?;
        routes.extend(result);
    }

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

/// List all routes filtered by a neighbor
pub async fn list_routes_filtered(
    Path(id): Path<String>,
) -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowRouteAllFilteredProtocol(id))?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, mut results_rx) = RoutesWorkerPool::spawn();

    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    });

    while let Some(result) = results_rx.recv().await {
        let result = result?;
        routes.extend(result);
    }

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };

    let body = serde_json::to_string(&response)?;
    Ok(body)
}

/// List all routes not exported
pub async fn list_routes_noexport(
    Path(id): Path<String>,
) -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowRouteAllNoexportProtocol(id))?;
    let buf = BufReader::new(result);
    let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
    let mut routes: Vec<Route> = vec![];

    // Spawn workers
    let (blocks_tx, mut results_rx) = RoutesWorkerPool::spawn();
    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    });

    while let Some(result) = results_rx.recv().await {
        let result = result?;
        routes.extend(result);
    }

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

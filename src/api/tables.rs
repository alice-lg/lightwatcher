use std::io::BufReader;

use anyhow::Result;
use axum::extract::Path;
use tokio::task;

use crate::{
    api::{responses::RoutesResponse, Error},
    bird,
    parsers::{
        parser::BlockIterator, routes::RE_ROUTES_START,
        routes_worker::RoutesWorkerPool,
    },
    state::Route,
};

/// List all routes in a table
pub async fn list_routes(Path(table): Path<String>) -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowRouteAllTable(table))?;
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

/// List all routes in a table
pub async fn list_routes_filtered(
    Path(table): Path<String>,
) -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowRouteAllFilteredTable(table))?;
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

use std::io::BufReader;

use anyhow::Result;
use axum::extract::Path;
use tokio::task;

use crate::{
    api::{responses::RoutesResponse, Error},
    bird,
    parsers::{parser::BlockIterator, routes::RE_ROUTES_START, routes_worker::RoutesWorkerPool},
    state::Route,
};

/// List all routes in a table
pub async fn list_routes(Path(table): Path<String>) -> Result<String, Error> {
    let result = bird::show_route_all_table(&table)?;
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

/// List all routes in a table
pub async fn list_routes_filtered(Path(table): Path<String>) -> Result<String, Error> {
    let result = bird::show_route_all_table_filtered(&table)?;
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

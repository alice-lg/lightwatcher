use anyhow::Result;
use axum::extract::Path;

use crate::{
    api::{responses::RoutesResponse, Error},
    bird::{Birdc, TableID},
};

/// List all routes in a table
pub async fn list_routes(Path(table): Path<String>) -> Result<RoutesResponse, Error> {
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
pub async fn list_routes_filtered(
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


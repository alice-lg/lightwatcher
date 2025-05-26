use anyhow::Result;
use axum::{
    extract::Path,
};

use crate::{
    api::{
        responses::{NeighborsResponse, RoutesResponse},
        Error,
    },
    bird::{Birdc, ProtocolID},
};

/// List all neighbors (show protocols all, filter BGP)
pub async fn list() -> Result<NeighborsResponse, Error> {
    let birdc = Birdc::default();
    let protocols = birdc.show_protocols_all().await?;
    let response = NeighborsResponse {
        protocols,
        ..Default::default()
    };
    Ok(response)
}

/// List all routes received for a neighbor
pub async fn list_routes_received(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;
    let routes = birdc.show_route_all_protocol(&protocol).await?;

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    Ok(response)
}

/// List all routes filtered by a neighbor
pub async fn list_routes_filtered(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;
    let routes = birdc.show_route_all_filtered_protocol(&protocol).await?;
    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    Ok(response)
}

/// List all routes not exported
pub async fn list_routes_noexport(
    Path(id): Path<String>,
) -> Result<RoutesResponse, Error> {
    let birdc = Birdc::default();
    let protocol = ProtocolID::parse(&id)?;
    let routes = birdc.show_route_all_noexport_protocol(&protocol).await?;

    let response = RoutesResponse {
        routes,
        ..Default::default()
    };
    Ok(response)
}

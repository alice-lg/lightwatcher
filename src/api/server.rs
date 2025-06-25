use anyhow::Result;
use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    api::{neighbors, status, tables},
    config,
};

/// Get the welcome message
async fn welcome() -> String {
    format!("lightwatcher {}", crate::version())
}

/// Start the API http server
pub async fn start() -> Result<()> {
    let app = Router::new()
        .route("/", get(welcome))
        .route("/status", get(status::retrieve))
        .route("/protocols/bgp", get(neighbors::list))
        .route(
            "/routes/received/:neighbor_id",
            get(neighbors::list_routes_received),
        )
        .route(
            "/routes/protocol/:neighbor_id",
            get(neighbors::list_routes_received),
        )
        .route(
            "/routes/filtered/:neighbor_id",
            get(neighbors::list_routes_filtered),
        )
        .route(
            "/routes/noexport/:neighbor_id",
            get(neighbors::list_routes_noexport),
        )
        .route("/routes/table/:table", get(tables::list_routes))
        .route(
            "/routes/table/:table/filtered",
            get(tables::list_routes_filtered),
        )
        .layer(TraceLayer::new_for_http());

    let listen = config::get_listen_address();
    let listener = TcpListener::bind(&listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

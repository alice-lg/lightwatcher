use anyhow::Result;
use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    api::{protocols, routes, status},
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
        .route("/protocols", get(protocols::list))
        .route("/protocols/bgp", get(protocols::list_bgp))
        .route(
            "/routes/received/:neighbor_id",
            get(routes::list_routes_received),
        )
        .route(
            "/routes/protocol/:neighbor_id",
            get(routes::list_routes_received),
        )
        .route(
            "/routes/filtered/:neighbor_id",
            get(routes::list_routes_filtered),
        )
        .route(
            "/routes/noexport/:neighbor_id",
            get(routes::list_routes_noexport),
        )
        .route("/routes/table/:table", get(routes::list_routes_table))
        .route(
            "/routes/table/:table/filtered",
            get(routes::list_routes_table_filtered),
        )
        .layer(TraceLayer::new_for_http());

    let listen = config::get_listen_address();
    let listener = TcpListener::bind(&listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

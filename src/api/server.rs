use anyhow::Result;
use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;

use crate::api::{neighbors, status, tables};

/// Server Options
#[derive(Default, Debug)]
pub struct Opts {
    /// Server listen address
    pub listen: String,
}

/// Get the welcome message
async fn welcome() -> &'static str {
    "lightwatcher v0.0.1"
}

/// Start the API http server
pub async fn start(opts: &Opts) -> Result<()> {
    let app = Router::new()
        .route("/", get(welcome))
        .route("/status", get(status::retrieve))
        .route("/protocols/bgp", get(neighbors::list))
        .route(
            "/routes/received/:neighbor_id",
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

    let listener = tokio::net::TcpListener::bind(&opts.listen).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

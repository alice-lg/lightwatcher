use anyhow::Result;
use axum::{
    extract::{Extension, Path},
    routing::get,
    Router,
};

use crate::api::{neighbors, status, tables, Error};

/// Server Options
#[derive(Default, Debug)]
pub struct Opts {
    /// Server listen address
    pub listen: String,

    /// Server workers
    pub workers: usize,
}

/// Get the welcome message
async fn welcome() -> &'static str {
    "lightwatcher v0.0.1"
}

/// Start the API http server
pub async fn start(opts: &Opts) -> Result<()> {
    let addr = opts.listen.parse()?;
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
        );
    //      .layer(Extension(store));

    println!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

use anyhow::Result;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Serialize;

use crate::api::status;
use crate::api::Error;
use crate::state::Status;

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
        .route("/status", get(status::retrieve));
    /*
    .route("/protocols/bgp", get(list_neighbors))
    .route("/routes/received/:neighbor_id", get(list_routes_recieved))
    .route("/routes/filtered/:neighbor_id", get(list_routes_filtered))
    .route("/routes/table/:table", get(list_routes_table))
    .route(
        "/routes/table/:table/filtered",
        get(list_routes_table_filtered),
    );
    */
    //      .layer(Extension(store));

    println!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

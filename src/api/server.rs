use anyhow::Result;
use axum::{middleware, routing::get, Extension, Router};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    normalize_path::NormalizePathLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
    api::{protocols, rate_limit, routes, status},
    config,
};

/// Get the welcome message
async fn welcome() -> String {
    format!("lightwatcher {}", crate::version())
}

/// Start the API http server
pub async fn start() -> Result<()> {
    // Create rate limiter
    let rate_limit_config = config::get_rate_limit_config();
    let rate_limiter = rate_limit::RateLimiter::new(rate_limit_config);

    let app = Router::new()
        .route("/", get(welcome))
        .route("/status", get(status::retrieve))
        .route("/protocols", get(protocols::list))
        .route("/protocols/bgp", get(protocols::list_bgp))
        .route(
            "/routes/received/{neighbor_id}",
            get(routes::list_routes_received),
        )
        .route(
            "/routes/protocol/{neighbor_id}",
            get(routes::list_routes_received),
        )
        .route(
            "/routes/filtered/{neighbor_id}",
            get(routes::list_routes_filtered),
        )
        .route(
            "/routes/noexport/{neighbor_id}",
            get(routes::list_routes_noexport),
        )
        .route("/routes/table/{table}", get(routes::list_routes_table))
        .route(
            "/routes/table/{table}/filtered",
            get(routes::list_routes_table_filtered),
        )
        .route(
            "/routes/table/{table}/peer/{peer}",
            get(routes::list_routes_table_peer),
        )
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(
            TraceLayer::new_for_http()
                .on_request(())
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(Extension(rate_limiter));

    let listen = config::get_listen_address();
    let listener = TcpListener::bind(&listen).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lightwatcher::{api, config};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "lightwatcher=info,axum::rejection=trace,tower_http=debug"
                        .into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Print info
    tracing::info!(version = lightwatcher::version(), "starting service");
    tracing::info!(LIGHTWATCHER_LISTEN = config::get_listen_address(), "env");
    tracing::info!(LIGHTWATCHER_BIRDC = config::get_birdc_socket(), "env");
    let cache = config::get_neighbors_cache_config();
    tracing::info!(
        LIGHTWATCHER_NEIGHBORS_CACHE_MAX_ENTRIES = cache.max_entries,
        LIGHTWATCHER_NEIGHBORS_CACHE_TTL = cache.ttl.num_seconds(),
        "env"
    );
    let cache = config::get_routes_cache_config();
    tracing::info!(
        LIGHTWATCHER_ROUTES_CACHE_MAX_ENTRIES = cache.max_entries,
        LIGHTWATCHER_ROUTES_CACHE_TTL = cache.ttl.num_seconds(),
        "env"
    );

    // Start API server
    api::server::start().await?;
    Ok(())
}

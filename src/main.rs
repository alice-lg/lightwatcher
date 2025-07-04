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

    // Print service info and configuration
    tracing::info!(version = lightwatcher::version(), "starting service");
    config::log_env();

    // Start API server
    api::server::start().await?;
    Ok(())
}

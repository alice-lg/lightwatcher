use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lightwatcher::api::{self, server::Opts};
use lightwatcher::config;

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
    tracing::info!("starting {}", lightwatcher::version());
    tracing::info!(
        "ENV: LIGHTWATCHER_LISTEN={}",
        config::get_listen_address()
    );
    tracing::info!("ENV: LIGHTWATCHER_BIRDC={}", config::get_birdc_socket());

    // Start API server
    let listen = config::get_listen_address();
    api::server::start(&Opts { listen }).await?;

    Ok(())
}

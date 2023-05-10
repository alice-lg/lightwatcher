use anyhow::Result;

use lightwatcher::api::{self, server::Opts};

#[tokio::main]
async fn main() -> Result<()> {
    // Start API server
    api::server::start(&Opts {
        listen: "127.0.0.1:8181".to_string(),
        workers: 8,
    })
    .await?;
    Ok(())
}

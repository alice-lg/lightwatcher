use anyhow::Result;

use lightwatcher::api;

#[tokio::main]
async fn main() -> Result<()> {
    // Start API server
    api::start_server(29184).await?;

    Ok(())
}

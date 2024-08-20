use anyhow::Result;

use lightwatcher::api::{self, server::Opts};
use lightwatcher::config;

#[tokio::main]
async fn main() -> Result<()> {
    // Print info
    let listen = config::get_listen_address();
    let birdc_socket = config::get_birdc_socket();

    println!("lightwatcher v0.0.1\n");
    println!("    LIGHTWATCHER_LISTEN: {}", listen);
    println!("    LIGHTWATCHER_BIRDC: {}", birdc_socket);
    println!("\n");

    // Start API server
    api::server::start(&Opts { listen }).await?;
    Ok(())
}

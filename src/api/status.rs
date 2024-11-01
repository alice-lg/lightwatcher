use anyhow::Result;

use crate::{
    api::{responses::StatusResponse, Error},
    bird::Birdc,
};

/// Get the current status
pub async fn retrieve() -> Result<String, Error> {
    let birdc = Birdc::default();
    let status = birdc.show_status().await?;
    let response = StatusResponse {
        status,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

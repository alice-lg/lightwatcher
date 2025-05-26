use anyhow::Result;

use crate::{
    api::{responses::StatusResponse, Error},
    bird::Birdc,
};

/// Get the current status
pub async fn retrieve() -> Result<StatusResponse, Error> {
    let birdc = Birdc::default();
    let status = birdc.show_status().await?;
    let response = StatusResponse {
        status,
        ..Default::default()
    };
    Ok(response)
}

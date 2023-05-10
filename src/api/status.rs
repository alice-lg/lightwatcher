use crate::api::Error;
use crate::state::{ApiStatus, BirdStatus, CacheInfo, CacheStatus, Status};

use anyhow::Result;

pub async fn retrieve() -> Result<String, Error> {
    let status = Status::default();
    let body = serde_json::to_string(&status)?;
    Ok(body)
}

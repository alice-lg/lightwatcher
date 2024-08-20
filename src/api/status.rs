use std::io::{BufRead, BufReader};

use anyhow::Result;

use crate::api::{responses::StatusResponse, Error};
use crate::bird;
use crate::parsers::parser::Parse;
use crate::state::{ApiStatus, BirdStatus};

/// Get the current status
pub async fn retrieve() -> Result<String, Error> {
    let result = bird::birdc(bird::Command::ShowStatus)?;
    let reader = BufReader::new(result);
    let block = reader.lines().map(|l| l.unwrap()).collect::<Vec<String>>();
    let status = BirdStatus::parse(block).unwrap();

    let response = StatusResponse {
        api: ApiStatus::default(),
        status,
        ..Default::default()
    };
    let body = serde_json::to_string(&response)?;
    Ok(body)
}

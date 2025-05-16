use std::{
    fmt::Display,
    io::{BufReader, Write},
    os::unix::net::UnixStream,
};

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;
use tokio::task;

use crate::{
    config,
    parsers::{
        neighbors::NeighborReader,
        parser::{BlockIterator, Parse},
        routes::RE_ROUTES_START,
        routes_worker::RoutesWorkerPool,
    },
    route_server::{BirdStatus, Neighbor, NeighborsMap, Route},
};

lazy_static! {
    /// Regex for start neighbor
    static ref RE_STATUS_START: Regex = Regex::new(r"\d\d\d\d\s").unwrap();
}

#[derive(Error, Debug)]
pub struct ValidationError {
    input: String,
    reason: String,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation failed '{}': {}", self.input, self.reason)
    }
}

// Validation helpers

/// Basic string validation
fn validate_string(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(ValidationError {
            input: s.to_string(),
            reason: "is empty".to_string(),
        }
        .into());
    }

    if s.len() > 128 {
        return Err(ValidationError {
            input: s.to_string(),
            reason: "is too long".to_string(),
        }
        .into());
    }

    // Only allow [a-zA-Z0-9_]
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ValidationError {
            input: s.to_string(),
            reason: "contains invalid characters".to_string(),
        }
        .into());
    }

    Ok(())
}

// Request Types

/// TableID represents a table name like master4
pub struct TableID(String);

impl TableID {
    /// Parse a table id from a string. This will fail
    /// if the input is invalid.
    pub fn parse(s: &str) -> Result<Self> {
        let table = s.to_string();
        validate_string(&table)?;

        Ok(Self(table))
    }

    /// Get the table id as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for TableID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ProtocolID represents a neighbor identifier.
/// Valid characters are [a-zA-Z0-9_].
pub struct ProtocolID(String);

impl ProtocolID {
    /// Parse a protocol id from a string. This will fail
    /// if the input is invalid.
    pub fn parse(s: &str) -> Result<Self> {
        let protocol = s.to_string();
        validate_string(&protocol)?;

        Ok(Self(protocol))
    }

    /// Get the protocol id as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ProtocolID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Birdc {
    socket: String,
}

impl Default for Birdc {
    fn default() -> Self {
        Self {
            socket: config::get_birdc_socket(),
        }
    }
}

impl Birdc {
    /// Create new birdc instance
    pub fn new(socket: String) -> Self {
        Self { socket }
    }

    /// Get the daemon status.
    pub async fn show_status(&self) -> Result<BirdStatus> {
        let mut stream = UnixStream::connect(&self.socket)?;

        let cmd = format!("show status\n");
        stream.write_all(&cmd.as_bytes())?;

        let reader = BufReader::new(stream);
        let mut iter = BlockIterator::new(reader, &RE_STATUS_START);
        let block = iter.next().unwrap();
        let status = BirdStatus::parse(block)?;

        Ok(status)
    }

    /// Get neighbors
    pub async fn show_protocols_all(&self) -> Result<NeighborsMap> {
        let mut stream = UnixStream::connect(&self.socket)?;
        let cmd = format!("show protocols all\n");
        stream.write_all(&cmd.as_bytes())?;

        let buf = BufReader::new(stream);
        let reader = NeighborReader::new(buf);
        let neighbors: Vec<Neighbor> =
            reader.filter(|n| !n.id.is_empty()).collect();

        let neighbors: NeighborsMap =
            neighbors.into_iter().map(|n| (n.id.clone(), n)).collect();

        Ok(neighbors)
    }

    /// Send the command to the birdc socket and parse the response.
    /// Please note that only show route commands can be used here.
    async fn fetch_routes_cmd(&self, cmd: &str) -> Result<Vec<Route>> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.write_all(&cmd.as_bytes())?;
        let buf = BufReader::new(stream);

        let blocks = BlockIterator::new(buf, &RE_ROUTES_START);
        let mut routes: Vec<Route> = vec![];

        // Spawn workers and fill queue
        let (blocks_tx, mut results_rx) = RoutesWorkerPool::spawn();
        task::spawn_blocking(move || {
            for block in blocks {
                blocks_tx.send(block).unwrap();
            }
        });

        // Collect results
        while let Some(result) = results_rx.recv().await {
            let result = result?;
            routes.extend(result);
        }

        Ok(routes)
    }

    /// Get routes for a table
    pub async fn show_route_all_table(
        &self,
        table: &TableID,
    ) -> Result<Vec<Route>> {
        let cmd = format!("show route all table '{}'\n", table);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get filtered routes for a table
    pub async fn show_route_all_filtered_table(
        &self,
        table: &TableID,
    ) -> Result<Vec<Route>> {
        let cmd = format!("show route all filtered table '{}'\n", table);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get routes for a neighbor
    pub async fn show_route_all_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<Vec<Route>> {
        let cmd = format!("show route all protocol '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get routes for a neighbor
    pub async fn show_route_all_filtered_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<Vec<Route>> {
        let cmd = format!("show route all filtered protocol '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get noexport routes for a neighbor
    pub async fn show_route_all_noexport_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<Vec<Route>> {
        // TODO: check command
        let cmd = format!("show route all noexport protocol '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_id() {
        let table = TableID::parse("master4").unwrap();
        assert_eq!(table.as_str(), "master4");

        // Invalid table name
        let result = TableID::parse("m4'");
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_id() {
        let protocol = ProtocolID::parse("R192_175").unwrap();
        assert_eq!(protocol.as_str(), "R192_175");

        // Invalid table name
        let result = ProtocolID::parse("R192_175'");
        assert!(result.is_err());

        let result = ProtocolID::parse("R192 175");
        assert!(result.is_err());

        let result = ProtocolID::parse("R192`date`175");
        assert!(result.is_err());
    }
}

use std::{
    collections::HashMap,
    fmt::Display,
    io::{BufReader, Write},
    os::unix::net::UnixStream,
};

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::{
    config,
    parsers::{
        parser::{BlockIterator, Parse},
        protocols::{ProtocolReader, ProtocolReceiver},
        routes::RE_ROUTES_START,
        routes_worker::{self, RoutesResultsReceiver},
    },
};

lazy_static! {
    /// Regex for start / stop status.
    static ref RE_STATUS_START: Regex = Regex::new(r"EOF").unwrap();
    static ref RE_STATUS_STOP: Regex = Regex::new(r"0013\s").unwrap();
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

    // Only allow [a-zA-Z0-9_.:]
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == ':')
    {
        return Err(ValidationError {
            input: s.to_string(),
            reason: "contains invalid characters".to_string(),
        }
        .into());
    }

    Ok(())
}

/// QueryValue represents a parameter that will be included
/// in the query sent to bird.
pub struct QueryValue(String);

impl QueryValue {
    /// Parse a query value from a string. This will fail
    /// if the input is invalid.
    pub fn parse(s: &str) -> Result<Self> {
        let table = s.to_string();
        validate_string(&table)?;

        Ok(Self(table))
    }

    /// Get the value as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for QueryValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type ProtocolID = QueryValue;
pub type TableID = QueryValue;
pub type PeerID = QueryValue;

/// Bird status
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BirdStatus {
    pub current_server: String,
    pub last_reboot: String,
    pub last_reconfig: String,
    pub message: String,
    pub router_id: String,
    pub version: String,
}

/// Routes count. This is a mapping of
///   "received", "rejected", "filtered", ...
pub type RoutesCount = HashMap<String, u32>;

/// Route change stats is a mapping of per channel stats
/// for attributes: received, rejected, filtered, ignored, ...
pub type RouteChangeStats = HashMap<String, Option<u32>>;

/// Change stats
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RouteChanges {
    pub import_updates: RouteChangeStats,
    pub import_withdraws: RouteChangeStats,
    pub export_updates: RouteChangeStats,
    pub export_withdraws: RouteChangeStats,
}

/// Channel
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Channel {
    pub state: String,
    pub import_state: String,
    pub export_state: String,
    pub table: String,
    pub peer_table: String,
    pub preference: u32,
    pub input_filter: String,
    pub output_filter: String,
    pub routes_count: RoutesCount,
    pub route_change_stats: RouteChanges,
    pub bgp_next_hop: String,
}

/// Per channel statistics
pub type ChannelMap = HashMap<String, Channel>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Protocol {
    pub id: String,
    pub bird_protocol: String,
    #[serde(rename = "neighbor_address")]
    pub address: String,
    #[serde(rename = "neighbor_as")]
    pub asn: u32,
    pub state: String,
    pub description: String,
    pub routes: RoutesCount,
    pub channels: ChannelMap,
    pub since: String,
    pub state_changed: String,
    pub last_error: String,
    // Compat
    pub table: String,
    pub peer_table: String,
}

pub type ProtocolsMap = HashMap<String, Protocol>;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Community(pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LargeCommunity(pub u32, pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ExtCommunity(pub String, pub String, pub String);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BGPInfo {
    pub origin: String,
    pub as_path: Vec<String>,
    pub next_hop: String,
    pub communities: Vec<Community>,
    pub large_communities: Vec<LargeCommunity>,
    pub ext_communities: Vec<ExtCommunity>,
    pub local_pref: String,
    pub med: String,
    pub otc: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Route {
    #[serde(rename = "from_protocol")]
    pub neighbor_id: Option<String>,
    pub network: String,
    pub interface: String,
    pub gateway: String,
    pub metric: u32,
    pub bgp: BGPInfo,
    pub age: String,
    #[serde(rename = "type")]
    pub route_type: Vec<String>,
    pub primary: bool,
    pub learnt_from: Option<String>,
}

/// A connection wraps a socket address and will produce
/// a socket connection of type UnixStream.
pub struct Connection {
    lock: mpsc::Sender<()>,
}

impl Connection {
    pub fn open(&self, addr: &str) -> Result<UnixStream> {
        let stream = UnixStream::connect(addr)?;
        Ok(stream)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let lock = self.lock.clone();
        tokio::spawn(async move {
            if let Err(e) = lock.send(()).await {
                panic!("send failed: {}", e);
            }
        });
    }
}

/// A ConnectionRequest is issued to the pool and will be
/// fulfilled if free slots are available.
type ConnectionRequest = oneshot::Sender<Connection>;

/// The ConnectionPool limits the amount of concurrent conncetions to the bird daemon. The
/// UnixStream connections are not reused in order to establish new sessions with the daemon.
#[derive(Clone)]
pub struct ConnectionPool {
    requests: mpsc::Sender<ConnectionRequest>,
}

impl ConnectionPool {
    /// Start a new connection pool
    pub fn start(limit: usize) -> Self {
        let (lock_tx, mut lock_rx) = mpsc::channel(limit);
        let (req_tx, mut req_rx) = mpsc::channel::<ConnectionRequest>(limit);

        tracing::info!(limit = limit, "starting bird connection pool");

        tokio::spawn(async move {
            let mut size = 0;
            while let Some(conn_tx) = req_rx.recv().await {
                if size > limit {
                    // Wait until a new connection slot is available.
                    if lock_rx.recv().await.is_some() {
                        size -= 1;
                    } else {
                        panic!("pool lock dropped");
                    }

                    // Drain pending closed connection.
                    while lock_rx.try_recv().is_ok() {
                        size -= 1;
                    }
                }

                let conn = Connection {
                    lock: lock_tx.clone(),
                };

                if conn_tx.send(conn).is_err() {
                    tracing::warn!("connection request dropped");
                    continue;
                }

                size += 1;
            }
        });

        ConnectionPool { requests: req_tx }
    }

    /// Try to acquire a new connection. This will block / await until
    /// a free slot is available.
    pub async fn acquire(&self) -> Connection {
        let (conn_tx, conn_rx) = oneshot::channel();
        self.requests.send(conn_tx).await.expect("must work");
        conn_rx.await.expect("this must work second")
    }
}

lazy_static! {
    /// A global connection pool is used to limit the amount
    /// of concurrent connections to the daemon.
    static ref BIRD_CONNECTION_POOL: ConnectionPool = {
        let limit = config::get_birdc_connection_pool_size();
        ConnectionPool::start(limit)
    };
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
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;

        let cmd = "show status\n";
        stream.write_all(cmd.as_bytes())?;

        let reader = BufReader::new(stream);
        let mut iter = BlockIterator::new(reader, &RE_STATUS_START)
            .with_stop(&RE_STATUS_STOP);
        let block = iter.next().unwrap();
        let status = BirdStatus::parse(block)?;

        Ok(status)
    }

    /// Get neighbors
    pub async fn show_protocols(&self) -> Result<ProtocolsMap> {
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;

        let cmd = "show protocols all\n";
        stream.write_all(cmd.as_bytes())?;

        let buf = BufReader::new(stream);
        let reader = ProtocolReader::new(buf);

        let protocols: Vec<Protocol> =
            reader.filter(|n| !n.id.is_empty()).collect();

        let protocols: ProtocolsMap =
            protocols.into_iter().map(|n| (n.id.clone(), n)).collect();

        Ok(protocols)
    }

    pub async fn show_protocols_stream(&self) -> Result<ProtocolReceiver> {
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;

        let cmd = "show protocols all\n";
        stream.write_all(cmd.as_bytes())?;

        let buf = BufReader::new(stream);
        let reader = ProtocolReader::new(buf);

        let protocols = reader.stream();
        Ok(protocols)
    }

    pub async fn show_protocols_bgp(&self) -> Result<ProtocolsMap> {
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;

        let cmd = "show protocols all\n";
        stream.write_all(cmd.as_bytes())?;

        let buf = BufReader::new(stream);
        let reader = ProtocolReader::new(buf).with_filter_bgp();
        let protocols: Vec<Protocol> =
            reader.filter(|n| !n.id.is_empty()).collect();

        let protocols: ProtocolsMap =
            protocols.into_iter().map(|n| (n.id.clone(), n)).collect();

        Ok(protocols)
    }

    pub async fn show_protocols_bgp_stream(&self) -> Result<ProtocolReceiver> {
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;
        let cmd = "show protocols all\n";
        stream.write_all(cmd.as_bytes())?;

        let buf = BufReader::new(stream);
        let reader = ProtocolReader::new(buf).with_filter_bgp();

        let protocols = reader.stream();
        Ok(protocols)
    }

    /// Send the command to the birdc socket and parse the response
    /// using the worker pool.
    ///
    /// Please note that only show route commands can be used here.
    async fn fetch_routes_cmd(
        &self,
        cmd: &str,
    ) -> Result<RoutesResultsReceiver> {
        let mut stream =
            BIRD_CONNECTION_POOL.acquire().await.open(&self.socket)?;
        stream.write_all(cmd.as_bytes())?;
        let buf = BufReader::new(stream);

        let blocks = BlockIterator::new(buf, &RE_ROUTES_START);

        // Spawn workers and fill queue
        let (results_tx, results) = mpsc::channel(64);

        tokio::spawn(async move {
            for block in blocks {
                if let Err(e) =
                    routes_worker::accept_block(block, results_tx.clone())
                        .await
                {
                    tracing::error!(
                        "routes worker failed accepting block: {}",
                        e
                    );
                    panic!();
                }
            }
        });

        Ok(results)
    }

    /// Get all routes
    pub async fn show_route_all(&self) -> Result<RoutesResultsReceiver> {
        let cmd = "show route all";
        let results = self.fetch_routes_cmd(cmd).await?;
        Ok(results)
    }

    /// Get routes for a table
    pub async fn show_route_all_table(
        &self,
        table: &TableID,
    ) -> Result<RoutesResultsReceiver> {
        let cmd = format!("show route all table '{}'\n", table);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get filtered routes for a table
    pub async fn show_route_all_filtered_table(
        &self,
        table: &TableID,
    ) -> Result<RoutesResultsReceiver> {
        let cmd = format!("show route all filtered table '{}'\n", table);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get routes for a neighbor
    pub async fn show_route_all_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<RoutesResultsReceiver> {
        let cmd = format!("show route all protocol '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get routes for a neighbor
    pub async fn show_route_all_filtered_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<RoutesResultsReceiver> {
        let cmd = format!("show route all filtered protocol '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get noexport routes for a neighbor
    pub async fn show_route_all_noexport_protocol(
        &self,
        protocol: &ProtocolID,
    ) -> Result<RoutesResultsReceiver> {
        // TODO: check command
        let cmd = format!("show route all noexport '{}'\n", protocol);
        let routes = self.fetch_routes_cmd(&cmd).await?;
        Ok(routes)
    }

    /// Get routes for table and peer
    pub async fn show_route_all_table_peer(
        &self,
        table: &ProtocolID,
        peer: &PeerID,
    ) -> Result<RoutesResultsReceiver> {
        let cmd =
            format!("show route all table '{}' where from={}\n", table, peer,);
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

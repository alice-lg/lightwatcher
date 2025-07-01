use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
pub struct Neighbor {
    pub id: String,
    #[serde(rename = "neighbor_address")]
    pub address: String,
    #[serde(rename = "neighbor_as")]
    pub asn: u32,
    pub state: String,
    pub description: String,
    pub routes: RoutesCount,
    pub channels: ChannelMap,
    pub uptime: f64, // seconds
    pub since: DateTime<Utc>,
    pub state_changed: String,
    pub last_error: String,
    // #[serde(rename = "routeserver_id")]
    // pub route_server_id: String,
}

pub type NeighborsMap = HashMap<String, Neighbor>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Community(pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LargeCommunity(pub u32, pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
    pub otc: String,
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

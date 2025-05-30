use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cache Information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheInfo {
    pub date: DateTime<Utc>,
    pub timezone_type: String,
    pub timezone: String,
}

impl Default for CacheInfo {
    fn default() -> Self {
        Self {
            date: Utc::now(),
            timezone_type: "UTC".into(),
            timezone: "UTC".into(),
        }
    }
}

/// Cache Status
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CacheStatus {
    pub cached_at: CacheInfo,
}

/// ApiStatus
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiStatus {
    #[serde(rename = "Version")]
    pub version: String,
    pub result_from_cache: bool,
    pub cache_status: CacheStatus,
}

impl Default for ApiStatus {
    fn default() -> Self {
        ApiStatus {
            version: "0.0.1".to_string(),
            result_from_cache: false,
            cache_status: CacheStatus::default(),
        }
    }
}

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Status {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub status: BirdStatus,
    pub ttl: DateTime<Utc>,
}

// TODO: These should be options
/*
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RoutesCount {
    pub imported: u32,
    pub filtered: u32,
    pub exported: u32,
    pub preferred: u32,
}
*/
pub type RoutesCount = HashMap<String, u32>;

/*
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RouteChangeStats {
    pub received: u32,
    pub rejected: u32,
    pub filtered: u32,
    pub ignored: u32,
    pub rx_limit: u32,
    pub limit: u32,
    pub accepted: u32,
}
*/
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
    // pub as_path: Vec<u32>,
    pub as_path: Vec<String>,
    pub next_hop: String,
    pub communities: Vec<Community>,
    pub large_communities: Vec<LargeCommunity>,
    pub ext_communities: Vec<ExtCommunity>,
    // pub local_pref: u32,
    pub local_pref: String,
    // pub med: u32,
    pub med: String,
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

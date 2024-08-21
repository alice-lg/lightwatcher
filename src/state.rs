use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cache Information
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CacheInfo {
    pub date: DateTime<Utc>,
    pub timezone_type: String,
    pub timezone: String,
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
    pub cache_status: Option<CacheStatus>,
}

impl Default for ApiStatus {
    fn default() -> Self {
        ApiStatus {
            version: "0.0.1".to_string(),
            result_from_cache: false,
            cache_status: None,
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RoutesCount {
    accepted: u32,
    exported: u32,
    filtered: u32,
    imported: u32,
    preferred: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Neighbor {
    pub id: String,
    pub address: String,
    pub asn: u32,
    pub state: String,
    pub description: String,
    pub routes: RoutesCount,
    pub uptime: f64, // seconds
    pub since: DateTime<Utc>,
    pub last_error: String,
    #[serde(rename = "routeserver_id")]
    pub route_server_id: String,

    pub routes_received: u32,
    pub routes_filtered: u32,
    pub routes_accepted: u32,
    pub routes_exported: u32,
}

pub type NeighborsMap = HashMap<String, Neighbor>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Community(pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LargeCommunity(pub u32, pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExtCommunity(pub String, pub u32, pub u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BGPInfo {
    pub origin: String,
    pub as_path: Vec<u32>,
    pub next_hop: String,
    pub communities: Vec<Community>,
    pub large_communities: Vec<LargeCommunity>,
    pub ext_communities: Vec<ExtCommunity>,
    pub local_pref: u32,
    pub med: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Route {
    pub neighbor_id: Option<String>,
    pub network: String,
    pub interface: String,
    pub gateway: String,
    pub metric: u32,
    pub bgp: BGPInfo,
    pub age: f64,
    #[serde(rename = "type")]
    pub route_type: Vec<String>,
    pub primary: bool,
    pub learnt_from: Option<String>,
}

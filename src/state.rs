use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Status {
    pub bird_version: String,
    pub bird_status: String,
    pub server_version: String,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Neighbor {
    pub id: String,
    pub address: String,
    pub asn: u32,
    pub state: String,
    pub description: String,
    pub routes_received: u32,
    pub routes_filtered: u32,
    pub routes_exported: u32,
    pub routes_preferred: u32,
    pub routes_accepted: u32,
    pub uptime: f64, // seconds
    pub since: DateTime<Utc>,
    pub last_error: String,
    #[serde(rename = "routeserver_id")]
    pub route_server_id: String,
}

/*
// BGPInfo is a set of BGP attributes
type BGPInfo struct {
    Origin           *string        `json:"origin"`
    AsPath           []int          `json:"as_path"`
    NextHop          *string        `json:"next_hop"`
    Communities      Communities    `json:"communities"`
    LargeCommunities Communities    `json:"large_communities"`
    ExtCommunities   ExtCommunities `json:"ext_communities"`
    LocalPref        int            `json:"local_pref"`
    Med              int            `json:"med"`
}

*/

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Community(u32, u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LargeCommunity(u32, u32, u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExtCommunity(String, u32, u32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BGPInfo {
    pub origin: Option<String>,
    pub as_path: Vec<u32>,
    pub next_hop: Option<String>,
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
    pub interface: Option<String>,
    pub gateway: Option<String>,
    pub metric: u32,
    pub bgp: Option<BGPInfo>,
    pub age: f64,
    #[serde(rename = "type")]
    pub route_type: Vec<String>,
    pub primary: bool,
    pub learnt_from: Option<String>,
}

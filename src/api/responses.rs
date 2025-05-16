use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::route_server::{ApiStatus, BirdStatus, Neighbor, Route};

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub status: BirdStatus,
    pub ttl: DateTime<Utc>,
}

impl Default for StatusResponse {
    fn default() -> Self {
        StatusResponse {
            api: ApiStatus::default(),
            cached_at: Utc::now(),
            status: BirdStatus::default(),
            ttl: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeighborsResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub protocols: HashMap<String, Neighbor>,
}

impl Default for NeighborsResponse {
    fn default() -> Self {
        NeighborsResponse {
            api: ApiStatus::default(),
            cached_at: Utc::now(),
            protocols: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoutesResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub routes: Vec<Route>,
}

impl Default for RoutesResponse {
    fn default() -> Self {
        RoutesResponse {
            api: ApiStatus::default(),
            cached_at: Utc::now(),
            routes: Vec::new(),
        }
    }
}

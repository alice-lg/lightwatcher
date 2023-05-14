use crate::state::{ApiStatus, BirdStatus, Neighbor, Route};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NeighborsResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub protocols: HashMap<String, Neighbor>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RoutesResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub routes: Vec<Route>,
}

use std::collections::HashMap;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    api::cache::CachedResponse,
    bird::{BirdStatus, Protocol, Route},
};

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

impl CachedResponse for ApiStatus {
    fn mark_cached(&mut self) {
        self.result_from_cache = true;
        self.cache_status = CacheStatus::default();
    }

    fn is_expired(&self) -> bool {
        let cached_at = &self.cache_status.cached_at.date;
        (Utc::now() - cached_at) > Duration::minutes(5)
    }

    fn get_cached_at(&self) -> DateTime<Utc> {
        let cached_at = &self.cache_status.cached_at.date;
        *cached_at
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl CachedResponse for StatusResponse {
    fn mark_cached(&mut self) {
        self.api.mark_cached();
        self.ttl = Utc::now() + Duration::minutes(5);
        self.cached_at = Utc::now();
    }

    fn is_expired(&self) -> bool {
        self.api.is_expired()
    }

    fn get_cached_at(&self) -> DateTime<Utc> {
        self.cached_at
    }
}

impl IntoResponse for StatusResponse {
    fn into_response(self) -> Response {
        Json::from(self).into_response()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProtocolsResponse {
    pub api: ApiStatus,
    pub cached_at: DateTime<Utc>,
    pub protocols: HashMap<String, Protocol>,
}

impl Default for ProtocolsResponse {
    fn default() -> Self {
        ProtocolsResponse {
            api: ApiStatus::default(),
            cached_at: Utc::now(),
            protocols: HashMap::new(),
        }
    }
}

impl CachedResponse for ProtocolsResponse {
    fn mark_cached(&mut self) {
        self.api.mark_cached();
        self.cached_at = Utc::now();
    }

    fn get_cached_at(&self) -> DateTime<Utc> {
        self.cached_at
    }

    fn is_expired(&self) -> bool {
        self.api.is_expired()
    }
}

impl IntoResponse for ProtocolsResponse {
    fn into_response(self) -> Response {
        Json::from(self).into_response()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl CachedResponse for RoutesResponse {
    fn mark_cached(&mut self) {
        self.api.mark_cached();
        self.cached_at = Utc::now();
    }

    fn is_expired(&self) -> bool {
        self.api.is_expired()
    }

    fn get_cached_at(&self) -> DateTime<Utc> {
        self.cached_at
    }
}

impl IntoResponse for RoutesResponse {
    fn into_response(self) -> Response {
        Json::from(self).into_response()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub bird_socket: String,
    pub bird_status: Option<BirdStatus>,
    pub error: Option<String>,
    pub bird_error: Option<String>,
}

impl IntoResponse for HealthResponse {
    fn into_response(self) -> Response {
        let status_code = if self.status == "ok" {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        };
        (status_code, Json(self)).into_response()
    }
}

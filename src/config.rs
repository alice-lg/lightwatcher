use std::{num::NonZeroUsize, thread};

use chrono::Duration;

/// The TTL and maximum number of entries can
/// be set in the CacheConfig.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl: Duration,
}

/// Get a string or default from env
fn string_from_env(key: &str, default: &str) -> String {
    let value = std::env::var(key).unwrap_or(default.to_string());
    value
}

/// Get the routes worker parallelism
pub fn get_routes_worker_pool_size() -> usize {
    let tap = thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap());

    match std::env::var("LIGHTWATCHER_ROUTES_WORKER_POOL_SIZE") {
        Err(_) => tap.get(),
        Ok(v) => v
            .parse()
            .expect("route workers pool size needs to be a valid number"),
    }
}

/// New cache config with ttl and max entries.
fn make_cache_config(max_entries: String, ttl: String) -> CacheConfig {
    let max_entries: usize = max_entries
        .parse()
        .expect("max entries must be a valid number");
    let ttl: i64 = ttl.parse().expect("ttl must be a valid number");
    let ttl = Duration::new(ttl, 0).expect("must be valid");

    CacheConfig { max_entries, ttl }
}

/// Get the configuration for the neighbors cache
pub fn get_neighbors_cache_config() -> CacheConfig {
    let max_entries =
        string_from_env("LIGHTWATCHER_NEIGHBORS_CACHE_MAX_ENTRIES", "1");
    let ttl = string_from_env("LIGHTWATCHER_NEIGHBORS_CACHE_TTL", "300");
    make_cache_config(max_entries, ttl)
}

/// Get the configuration for the routes cache
pub fn get_routes_cache_config() -> CacheConfig {
    let max_entries =
        string_from_env("LIGHTWATCHER_ROUTES_CACHE_MAX_ENTRIES", "25");
    let ttl = string_from_env("LIGHTWATCHER_ROUTES_CACHE_TTL", "300");
    make_cache_config(max_entries, ttl)
}

/// Get the birdc socket path from the environment
/// or use the default value.
pub fn get_birdc_socket() -> String {
    let socket = std::env::var("LIGHTWATCHER_BIRDC")
        .unwrap_or("/var/run/bird/bird.ctl".to_string());
    socket
}

pub fn get_listen_address() -> String {
    let listen = std::env::var("LIGHTWATCHER_LISTEN")
        .unwrap_or("127.0.0.1:8181".to_string());
    listen
}

/// Dump the current environment into the log.
pub fn log_env() {
    tracing::info!(LIGHTWATCHER_LISTEN = get_listen_address(), "env");
    tracing::info!(LIGHTWATCHER_BIRDC = get_birdc_socket(), "env");
    let cache = get_neighbors_cache_config();
    tracing::info!(
        LIGHTWATCHER_NEIGHBORS_CACHE_MAX_ENTRIES = cache.max_entries,
        LIGHTWATCHER_NEIGHBORS_CACHE_TTL = cache.ttl.num_seconds(),
        "env"
    );
    let cache = get_routes_cache_config();
    tracing::info!(
        LIGHTWATCHER_ROUTES_CACHE_MAX_ENTRIES = cache.max_entries,
        LIGHTWATCHER_ROUTES_CACHE_TTL = cache.ttl.num_seconds(),
        "env"
    );
    tracing::info!(
        LIGHTWATCHER_ROUTES_WORKER_POOL_SIZE = get_routes_worker_pool_size(),
        "env"
    );
}

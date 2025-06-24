use chrono::Duration;

/// The TTL and maximum number of entries can
/// be set in the CacheConfig.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl: Duration,
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

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::bird::ProtocolID;
use crate::config::CacheConfig;

/// Cached response provides a function for setting
/// the cache info metadata.
pub trait CachedResponse {
    fn mark_cached(&mut self);
    fn is_expired(&self) -> bool;
    fn get_cached_at(&self) -> DateTime<Utc>;
}

/// A key is a unique identifier for the cache
#[derive(Debug, Clone, Hash)]
pub struct CacheKey(String);

impl From<&str> for CacheKey {
    fn from(s: &str) -> Self {
        CacheKey(s.into())
    }
}

impl From<String> for CacheKey {
    fn from(s: String) -> Self {
        CacheKey(s)
    }
}

impl From<&ProtocolID> for CacheKey {
    fn from(p: &ProtocolID) -> Self {
        CacheKey(p.as_str().into())
    }
}

impl From<&CacheKey> for CacheKey {
    fn from(k: &CacheKey) -> Self {
        k.clone()
    }
}

/// Cache a response
#[derive(Debug, Clone)]
pub struct ResponseCache<T> {
    responses: HashMap<String, T>,
    config: CacheConfig,
}

impl<T> ResponseCache<T>
where
    T: CachedResponse + Clone,
{
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            responses: HashMap::new(),
        }
    }

    /// Insert an entry identified by key
    /// This modifies the cache metadata in the response
    pub fn put(&mut self, key: impl Into<CacheKey>, mut value: T) {
        let key: CacheKey = key.into();
        let key = key.0;
        value.mark_cached();
        self.responses.insert(key, value);

        // Evict if expired or if max entries is exceeded
        if self.responses.len() > self.config.max_entries {
            self.evict_expired();
        }
        if self.responses.len() > self.config.max_entries {
            self.evict_oldest();
        }
    }

    /// Retrieve an entry identified by key from cache
    pub fn get(&self, key: impl Into<CacheKey>) -> Option<&T> {
        let key: CacheKey = key.into();
        let key = key.0;
        if let Some(value) = self.responses.get(&key) {
            match value.is_expired() {
                true => None,
                false => Some(value),
            }
        } else {
            None
        }
    }

    /// Remove expired entries
    fn evict_expired(&mut self) {
        let mut keys: Vec<String> = vec![];
        for (key, res) in &self.responses {
            if res.is_expired() {
                keys.push(key.to_owned());
            }
        }
        for k in keys {
            self.responses.remove(&k);
        }
    }

    /// Remove the oldest entry
    fn evict_oldest(&mut self) {
        let mut remove_key = "".to_string();
        let mut remove_cached_at = Utc::now();

        for (k, res) in &self.responses {
            let cached_at = res.get_cached_at();
            if cached_at < remove_cached_at {
                remove_key = k.to_string();
                remove_cached_at = cached_at
            }
        }

        self.responses.remove(&remove_key);
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::api::responses::StatusResponse;

    fn get_cache_config() -> CacheConfig {
        CacheConfig {
            ttl: Duration::new(300, 0).unwrap(),
            max_entries: 2,
        }
    }

    #[test]
    fn test_cache_key_from() {
        let key: CacheKey = "foo".into();
        assert_eq!(key.0, "foo");
    }

    #[test]
    fn test_cache_get_set() {
        let mut cache =
            ResponseCache::<StatusResponse>::new(get_cache_config());
        let res = StatusResponse::default();

        cache.put("res", res.clone());

        let res = cache.get("res").unwrap();
        assert_eq!(res.api.result_from_cache, true)
    }
}

use std::collections::HashMap;

use lazy_static::lazy_static;

/// Cached response provides a function for setting
/// the cache info metadata.
pub trait CachedResponse {
    fn mark_cached(&mut self);
    fn is_expired(&self) -> bool;
}

/// A key is a unique identifier for the cache
#[derive(Debug, Clone, Hash)]
pub struct CacheKey(String);

impl From<&str> for CacheKey {
    fn from(s: &str) -> Self {
        CacheKey(s.into())
    }
}

/// Cache a response
#[derive(Debug, Clone)]
pub struct ResponseCache<T> {
    responses: HashMap<String, T>,
}

impl<T> ResponseCache<T>
where
    T: CachedResponse,
{
    pub fn new() -> Self {
        Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::responses::StatusResponse;

    #[test]
    fn test_cache_key_from() {
        let key: CacheKey = "foo".into();
        assert_eq!(key.0, "foo");
    }

    #[test]
    fn test_cache_get_set() {
        let mut cache = ResponseCache::<StatusResponse>::new();
        let res = StatusResponse::default();

        cache.put("res", res);

        let res = cache.get("res").unwrap();
        assert_eq!(res.api.result_from_cache, true)
    }
}

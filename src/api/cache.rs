use std::collections::HashMap;

/// A key is a unique identifier for the cache
#[derive(Clone)]
pub struct CacheKey(String);

/// Cache a response
#[derive(Clone)]
pub struct ResponseCache<T> {
    responses: HashMap<CacheKey, T>,
}

impl<T> ResponseCache<T> {
    /// Retrieve an entry identified by key from cache
    pub fn get(&self, key: CacheKey) -> Option<T> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}

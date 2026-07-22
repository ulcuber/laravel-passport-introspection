use std::num::NonZeroUsize;

use lru::LruCache;
use parking_lot::Mutex;

#[derive(Debug)]
pub struct LruCacheWrapper<K: std::hash::Hash + std::cmp::Eq, V> {
    inner: Mutex<LruCache<K, V>>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruCacheWrapper<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
        }
    }

    pub fn len(&self) -> usize {
        let cache = self.inner.lock();
        cache.len()
    }

    pub fn has(&self, key: &K) -> bool {
        let cache = self.inner.lock();
        cache.contains(key)
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.lock();
        cache.get(key).cloned()
    }

    pub fn put(&self, key: K, value: V) {
        let mut cache = self.inner.lock();
        cache.put(key, value);
    }

    pub fn remove(&self, key: &K) {
        let mut cache = self.inner.lock();
        cache.pop(key);
    }

    pub fn clear(&self) {
        let mut cache = self.inner.lock();
        cache.clear();
    }
}

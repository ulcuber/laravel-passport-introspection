use std::collections::HashSet;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::jwt::JWTClaims;
use crate::lru::LruCacheWrapper;

pub struct TokenCache {
    valid: LruCacheWrapper<String, (JWTClaims, Instant)>,
    invalid: LruCacheWrapper<String, bool>,
    computing: Mutex<HashSet<String>>,
    ttl: Duration,
}

impl TokenCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self {
        let half = capacity / 2;
        Self {
            valid: LruCacheWrapper::new(half),
            invalid: LruCacheWrapper::new(half),
            computing: Mutex::new(HashSet::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, token: &str) -> Option<JWTClaims> {
        if self.invalid.has(&token.to_string()) {
            return None;
        }

        if let Some((claims, cached_at)) = self.valid.get(&token.to_string()) {
            if cached_at.elapsed() >= self.ttl {
                self.valid.remove(&token.to_string());
                return None;
            }

            let now = chrono::Utc::now().timestamp() as f64;
            if claims.exp < now {
                self.valid.remove(&token.to_string());
                self.invalid.put(token.to_string(), true);
                return None;
            }

            return Some(claims);
        }

        None
    }

    pub async fn lock(&self, token: &str) -> Option<JWTClaims> {
        let already_computing = {
            let mut computing = self.computing.lock();
            !computing.insert(token.to_string())
        };

        if already_computing {
            for i in 0..50 {
                tokio::time::sleep(Duration::from_micros(100)).await;
                if let Some(claims) = self.get(token) {
                    tracing::debug!("Got computed from another thread in {} attempts", i);
                    return Some(claims);
                }
            }
        }

        None
    }

    pub fn unlock(&self, token: &str) {
        self.computing.lock().remove(token);
    }

    pub fn put_valid(&self, token: &str, claims: JWTClaims) {
        self.invalid.remove(&token.to_string());
        self.valid.put(token.to_string(), (claims, Instant::now()));
    }

    pub fn put_invalid(&self, token: &str) {
        self.valid.remove(&token.to_string());
        self.invalid.put(token.to_string(), true);
    }

    pub fn len(&self) -> usize {
        self.valid.len() + self.invalid.len()
    }

    pub fn clear(&self) {
        self.valid.clear();
        self.invalid.clear();
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new(1000, 60)
    }
}

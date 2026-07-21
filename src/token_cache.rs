use std::time::{Duration, Instant};

use crate::jwt::JWTClaims;
use crate::lru::LruCacheWrapper;

pub struct TokenCache {
    valid: LruCacheWrapper<String, (JWTClaims, Instant)>,
    invalid: LruCacheWrapper<String, Instant>,
    ttl: Duration,
}

impl TokenCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self {
        let half = capacity / 2;
        Self {
            valid: LruCacheWrapper::new(half),
            invalid: LruCacheWrapper::new(half),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, token: &str) -> Option<JWTClaims> {
        if let Some(cached_at) = self.invalid.get(&token.to_string()) {
            if cached_at.elapsed() < self.ttl {
                return None;
            } else {
                self.invalid.remove(&token.to_string());
            }
        }

        if let Some((claims, cached_at)) = self.valid.get(&token.to_string()) {
            if cached_at.elapsed() >= self.ttl {
                self.valid.remove(&token.to_string());
                return None;
            }

            let now = chrono::Utc::now().timestamp() as f64;
            if claims.exp < now {
                self.valid.remove(&token.to_string());
                self.invalid.put(token.to_string(), Instant::now());
                return None;
            }

            return Some(claims);
        }

        None
    }

    pub fn put_valid(&self, token: &str, claims: JWTClaims) {
        self.invalid.remove(&token.to_string());
        self.valid.put(token.to_string(), (claims, Instant::now()));
    }

    pub fn put_invalid(&self, token: &str) {
        self.valid.remove(&token.to_string());
        self.invalid.put(token.to_string(), Instant::now());
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

#![cfg(feature = "proxy")]

use std::fs;
use std::collections::HashMap;

use serde::Deserialize;

use crate::lru::LruCacheWrapper;

#[derive(Debug, Deserialize)]
struct Route {
    pub prefix: String,
    pub target: String,
    #[serde(default)]
    pub strip_prefix: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProxyConfigStructure {
    routes: Vec<Route>,
}

impl Default for ProxyConfigStructure {
    fn default() -> Self {
        Self {
            routes: vec![
                Route {
                    prefix: "/api/".to_string(),
                    // default Laravel Octane
                    target: "http://localhost:8000/".to_string(),
                    strip_prefix: false,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct ProxyConfig {
    routes: HashMap<String, Route>,
    cache: LruCacheWrapper<String, String>,
}

impl ProxyConfig {
    pub fn new() -> Self {
        let config_path = std::env::var("PROXY_CONFIG")
            .unwrap_or_else(|_| "proxy.toml".to_string());

        let raw = match fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str::<ProxyConfigStructure>(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        tracing::error!("Failed to parse proxy config: {}", e);
                        tracing::info!("Using default configuration");
                        ProxyConfigStructure::default()
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read config file {}: {}", config_path, e);
                tracing::info!("Using default configuration");
                ProxyConfigStructure::default()
            }
        };

        let mut routes = HashMap::with_capacity(raw.routes.len());
        for route in raw.routes {
            routes.insert(route.prefix.clone(), route);
        }

        let max_cache_size = std::env::var("PROXY_ROUTE_CACHE_SIZE")
            .unwrap_or_else(|_| "10000".to_string())
            .parse()
            .unwrap_or(10000);

        Self {
            routes,
            cache: LruCacheWrapper::new(max_cache_size),
        }
    }

    #[inline]
    pub fn route_request(&self, path: &str) -> String {
        if let Some(target) = self.cache.get(&path.to_string()) {
            tracing::debug!("Route cache hit: {} → {}", path, target);
            return target;
        }

        let mut matched_route: Option<&Route> = None;
        let mut matched_prefix: Option<&str> = None;
        let mut longest_match = 0;

        for (prefix, route) in self.routes.iter() {
            if path.starts_with(prefix) && prefix.len() > longest_match {
                longest_match = prefix.len();
                matched_route = Some(route);
                matched_prefix = Some(prefix);
            }
        }

        let result = if let Some(route) = matched_route {
            let suffix = if route.strip_prefix {
                path.strip_prefix(matched_prefix.unwrap()).unwrap_or("")
            } else {
                path
            };
            format!("{}{}", route.target, suffix)
        } else {
            format!("http://localhost{}", path)
        };

        tracing::debug!("Route match: {} → {}", path, result);

        self.cache.put(path.to_string(), result.clone());

        result
    }
}

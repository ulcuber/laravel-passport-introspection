#![cfg(feature = "proxy")]

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    routing::any,
    middleware::from_fn_with_state,
};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;

use crate::app::AppState;
use crate::config::Config;
use crate::database::AnyAccessTokenRepository;
use crate::middlewares::auth_middleware;
use crate::token_cache::TokenCache;

use super::proxy_handler;

pub struct MonoProxyAppState {
    pub target: Arc<String>,
    pub client: Arc<Client<HttpConnector, Body>>,
}

pub async fn create_app(
    config: Config,
    access_tokens_repository: AnyAccessTokenRepository,
    client: Client<HttpConnector, Body>,
) -> Router {
    let token_cache = TokenCache::new(config.token_cache_size, config.token_cache_ttl);

    let target = std::env::var("MONO_PROXY_TARGET")
        .unwrap_or_else(|_| "http://localhost".to_string())
        .trim()
        .to_string();

    let state = Arc::new(MonoProxyAppState {
        target: Arc::new(target),
        client: Arc::new(client),
    });

    let auth_state = Arc::new(AppState {
        config: Arc::new(config),
        access_tokens: Arc::new(access_tokens_repository),
        token_cache: Arc::new(token_cache),
    });

    Router::new()
        .route("/{*wildcard}", any(proxy_handler))
        .layer(from_fn_with_state(auth_state, auth_middleware))
        .with_state(state)
}

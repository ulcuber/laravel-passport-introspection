use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};
use tracing_subscriber::{
    EnvFilter,
    layer::SubscriberExt,
    util::{SubscriberInitExt, TryInitError},
};

use crate::config::Config;
use crate::database::{AnyAccessTokenRepository, create_token_repository};
use crate::jwt::init_crypto;
use crate::token_cache::TokenCache;

use crate::middlewares::{GatewayMiddlewareState, gateway_middleware};
use crate::routes::{introspect_form_handler, introspect_http_handler, introspect_json_handler};

pub fn setup_logging(binary_name: &'static str) -> Result<(), TryInitError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let package_name = env!("CARGO_PKG_NAME");
        let fallback_filter = format!("{}=info,{}=info", package_name, binary_name);

        EnvFilter::new(&fallback_filter)
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
}

pub struct AppState {
    pub config: Arc<Config>,
    pub access_tokens: Arc<AnyAccessTokenRepository>,
    pub token_cache: Arc<TokenCache>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let config = Config::from_env(Some(".env.introspector"))
            .map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

        let alg = config
            .get_algorithm()
            .expect("Invalid JWT_ALGORITHM in config");
        init_crypto(&config.jwt_public_key, alg, &config.client_id)
            .expect("Failed to initialize crypto");

        let access_tokens_repository = create_token_repository(
            &config.database_url,
            config.database_min_connections,
            config.database_max_connections,
        )
        .await?;

        let token_cache = TokenCache::new(config.token_cache_size, config.token_cache_ttl);

        Ok(Self {
            config: Arc::new(config),
            access_tokens: Arc::new(access_tokens_repository),
            token_cache: Arc::new(token_cache),
        })
    }
}

pub async fn create_app(
    config: Config,
    access_tokens_repository: AnyAccessTokenRepository,
) -> Router {
    let token_cache = TokenCache::new(config.token_cache_size, config.token_cache_ttl);

    let gateway_state = Arc::new(GatewayMiddlewareState {
        gateway_secret: Arc::new(config.gateway_secret.clone()),
    });
    let state = Arc::new(AppState {
        config: Arc::new(config),
        access_tokens: Arc::new(access_tokens_repository),
        token_cache: Arc::new(token_cache),
    });

    Router::new()
        .route("/introspect", post(introspect_form_handler))
        .route("/introspect-json", post(introspect_json_handler))
        .layer(from_fn_with_state(gateway_state, gateway_middleware))
        // No middleware. Gives special empty 403 instead as 401 reserved for token
        .route("/introspect-http", get(introspect_http_handler))
        .with_state(state)
}

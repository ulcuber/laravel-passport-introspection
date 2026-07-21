#![cfg(feature = "proxy")]

use tokio::net::TcpListener;
use anyhow::{Context, Result};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use laravel_passport_introspection::{
    config::Config,
    jwt::init_crypto,
    app::setup_logging,
    mono_proxy::create_app,
    database::create_token_repository,
};

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging(module_path!())?;

    let config = Config::from_env(None).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    let alg = config.get_algorithm().expect("Invalid JWT_ALGORITHM in config");
    init_crypto(&config.jwt_public_key, alg, &config.client_id).expect("Failed to initialize crypto");

    let repo = create_token_repository(
        &config.database_url, config.database_min_connections, config.database_max_connections,
    ).await?;
    let client = Client::builder(TokioExecutor::new())
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build_http();

    let app = create_app(config.clone(), repo, client).await;

    let port = config.server_port;
    let host = config.server_host;
    tracing::info!("Starting proxy service on {}:{}", host, port);

    let listener = TcpListener::bind(format!("{}:{}", host, port))
        .await
        .with_context(|| format!("Failed to bind {}:{}", host, port))?;
    axum::serve(listener, app).await?;

    Ok(())
}

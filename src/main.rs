use anyhow::{Context, Result};
use tokio::net::TcpListener;

use laravel_passport_introspection::{
    app::{create_app, setup_logging},
    config::Config,
    database::create_token_repository,
    jwt::init_crypto,
};

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging(module_path!())?;

    let config =
        Config::from_env(None).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    let alg = config
        .get_algorithm()
        .expect("Invalid JWT_ALGORITHM in config");
    init_crypto(&config.jwt_public_key, alg, &config.client_id)
        .expect("Failed to initialize crypto");

    let repo = create_token_repository(
        &config.database_url,
        config.database_min_connections,
        config.database_max_connections,
    )
    .await?;
    let app = create_app(config.clone(), repo).await;

    let port = config.server_port;
    let host = config.server_host;
    let listener = TcpListener::bind(format!("{}:{}", host, port))
        .await
        .with_context(|| format!("Failed to bind {}:{}", host, port))?;

    tracing::info!("Starting introspection service on {}:{}", host, port);
    axum::serve(listener, app).await?;

    Ok(())
}

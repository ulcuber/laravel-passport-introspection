use std::env;
use std::path::Path;

use tokio::net::UnixListener;
use tracing::info;
use anyhow::{Context, Result};

use laravel_passport_introspection::{
    config::Config,
    jwt::init_crypto,
    app::{setup_logging, create_app},
    database::create_token_repository,
};

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging(module_path!())?;

    let config = Config::from_env(None).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    let socket_path = env::var("SOCKET_PATH").unwrap_or_else(|_| "/tmp/introspector.sock".to_string()).trim().to_string();
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)
            .with_context(|| format!("Failed to remove old socket: {}", &socket_path))?;
    }

    let alg = config.get_algorithm().expect("Invalid JWT_ALGORITHM in config");
    init_crypto(&config.jwt_public_key, alg, &config.client_id).expect("Failed to initialize crypto");

    let repo = create_token_repository(
        &config.database_url, config.database_min_connections, config.database_max_connections,
    ).await?;
    let app = create_app(config.clone(), repo).await;

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("Failed to bind Unix socket: {}", &socket_path))?;

    info!("Introspection service listening on unix:{}", &socket_path);
    axum::serve(listener, app).await?;

    Ok(())
}

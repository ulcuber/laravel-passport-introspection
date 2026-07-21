use std::env;
use std::sync::Arc;
use std::path::Path;

use tokio::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::json;
use tracing::{info, error, debug};

use laravel_passport_introspection::{
    app::setup_logging,
    config::Config,
    database::{create_token_repository, AnyAccessTokenRepository, AccessTokenRepository},
    jwt::{init_crypto, validate_jwt},
    token_cache::TokenCache,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging(module_path!())?;

    let config = Config::from_env(None).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

    let socket_path = env::var("SOCKET_PATH").unwrap_or_else(|_| "/tmp/introspector.sock".to_string()).trim().to_string();
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    info!("Listening on unix socket: {}", &socket_path);

    let alg = config.get_algorithm().expect("Invalid JWT_ALGORITHM in config");
    init_crypto(&config.jwt_public_key, alg, &config.client_id).expect("Failed to initialize crypto");

    let repo = create_token_repository(
        &config.database_url, config.database_min_connections, config.database_max_connections,
    ).await?;
    let token_cache = Arc::new(TokenCache::new(config.token_cache_size, config.token_cache_ttl));

    loop {
        let (mut stream, _) = listener.accept().await?;
        let repo = repo.clone();
        let token_cache = token_cache.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let request = String::from_utf8_lossy(&buf[..n]);
                        let response = handle_request(&repo, &token_cache, &request).await;

                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                            tracing::error!("Failed to write response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Read error: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

async fn handle_request(repo: &AnyAccessTokenRepository, token_cache: &TokenCache, request: &str) -> String {
    let payload: serde_json::Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(_) => {
            return json!({ "message": "Invalid JSON" }).to_string();
        }
    };

    let token = match payload.get("token").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return json!({ "message": "Missing token" }).to_string();
        },
    };

    match token_cache.get(&token) {
        Some(claims) => {
            debug!("Token cache hit (valid)");
            return json!({
                "active": true,
                "sub": claims.sub,
            }).to_string();
        }
        None => {
            debug!("Token cache miss, validating JWT");

            let claims = match validate_jwt(&token, None) {
                Ok(c) => c,
                Err(e) => {
                    debug!("{}", e);
                    token_cache.put_invalid(&token);
                    return json!({ "active": false }).to_string();
                }
            };

            match repo.is_token_revoked(&claims.jti).await {
                Ok(false) => {
                    token_cache.put_valid(&token, claims.clone());

                    return json!({
                        "active": true,
                        "sub": claims.sub,
                    }).to_string();
                }
                Ok(true) => {
                    token_cache.put_invalid(&token);

                    return json!({ "active": false }).to_string();
                }
                Err(e) => {
                    error!("Database error while checking token: {}", e);
                    return json!({ "message": "Database error" }).to_string();
                }
            }
        }
    }
}

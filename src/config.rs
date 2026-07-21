use std::env;
use std::fs;

use dotenvy::{dotenv, from_filename_override};
use serde::Deserialize;
use jsonwebtoken::Algorithm;
use anyhow::{Context, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_min_connections: u32,
    pub database_max_connections: u32,
    pub gateway_secret: String,
    pub jwt_public_key: String,
    pub jwt_algorithm: String,
    pub server_port: u16,
    pub server_host: String,
    pub client_id: Option<String>,
    pub token_cache_size: usize,
    pub token_cache_ttl: u64,
}

impl Config {
    pub fn from_env(env_file: Option<&str>) -> Result<Self> {
        if let Some(file) = env_file {
            from_filename_override(file).with_context(|| format!("Failed to load {}", file))?;
        } else {
            dotenv().context("Failed to load .env")?;
        }

        fn validate_non_empty(value: &str, name: &str) -> Result<String> {
            if value.is_empty() {
                anyhow::bail!("{} must not be empty", name);
            }
            Ok(value.to_string())
        }

        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL must be set")?
            .trim()
            .to_string();
        validate_non_empty(&database_url, "DATABASE_URL")?;

        let database_min_connections = env::var("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DATABASE_MIN_CONNECTIONS must be a valid integer (1-65535)")?;

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u32>()
            .context("DATABASE_MAX_CONNECTIONS must be a valid integer (1-65535)")?;

        let gateway_secret = env::var("GATEWAY_SECRET")
            .context("GATEWAY_SECRET must be set")?
            .trim()
            .to_string();
        validate_non_empty(&gateway_secret, "GATEWAY_SECRET")?;

        let jwt_public_key = (match env::var("JWT_PUBLIC_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => {
                let key_path = env::var("JWT_PUBLIC_KEY_PATH")
                    .context("JWT_PUBLIC_KEY_PATH must be set or JWT_PUBLIC_KEY provided")?;
                validate_non_empty(&key_path, "JWT_PUBLIC_KEY_PATH")?;
                fs::read_to_string(&key_path)
                    .with_context(|| format!("Failed to read JWT public key from {}", key_path))?
            }
        }).trim().to_string();
        validate_non_empty(&jwt_public_key, "JWT_PUBLIC_KEY or JWT_PUBLIC_KEY_PATH file content")?;

        let jwt_algorithm = env::var("JWT_ALGORITHM")
            .unwrap_or_else(|_| "RS256".to_string())
            .trim()
            .to_string();

        let server_host = env::var("SERVER_HOST")
            .context("SERVER_HOST must be set")?
            .trim()
            .to_string();
        validate_non_empty(&server_host, "SERVER_HOST")?;

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .context("SERVER_PORT must be a valid port number (1-65535)")?;

        let client_id = env::var("CLIENT_ID").ok();

        let token_cache_size = env::var("TOKEN_CACHE_SIZE")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map(|v| if v < 2 { 2 } else { v }) // for 2 std::num::NonZeroUsize
            .unwrap_or(1000);
        let token_cache_ttl = env::var("TOKEN_CACHE_TTL")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .unwrap_or(60);

        Ok(Config {
            database_url,
            database_min_connections,
            database_max_connections,
            gateway_secret,
            jwt_public_key,
            jwt_algorithm,
            server_port,
            server_host,
            client_id,
            token_cache_size,
            token_cache_ttl,
        })
    }

    pub fn get_algorithm(&self) -> Result<Algorithm> {
        match self.jwt_algorithm.as_str() {
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "RS512" => Ok(Algorithm::RS512),
            "ES256" => Ok(Algorithm::ES256),
            "ES384" => Ok(Algorithm::ES384),
            "PS256" => Ok(Algorithm::PS256),
            "PS384" => Ok(Algorithm::PS384),
            "PS512" => Ok(Algorithm::PS512),
            "EdDSA" => Ok(Algorithm::EdDSA),
            "HS256" => Ok(Algorithm::HS256),
            "HS384" => Ok(Algorithm::HS384),
            "HS512" => Ok(Algorithm::HS512),
            _ => anyhow::bail!("Unsupported JWT algorithm: {}", self.jwt_algorithm),
        }
    }
}

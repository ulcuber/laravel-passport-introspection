use std::env;
use std::fs;
use std::io::Write;

use anyhow::{anyhow, Result, Context};
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
use uuid::Uuid;

use laravel_passport_introspection::{
    config::Config,
    database::{create_token_repository, AccessTokenRepository},
    jwt::{JWTClaims, TokenMeta},
};

#[tokio::main]
async fn main() -> Result<()> {
    // already loads .env
    let config = Config::from_env(None).map_err(|e| anyhow!("Configuration error: {}", e))?;

    let alg = config.get_algorithm().expect("Invalid JWT_ALGORITHM in config");
    let client_id = config.client_id.unwrap_or_else(|| "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75".to_string());
    let path = format!("wrk/tokens_{}.txt", config.jwt_algorithm);

    let token_count: usize = env::var("TOKEN_COUNT").unwrap_or_else(|_| "1000".to_string()).parse()?;
    let private_key_path = env::var("JWT_PRIVATE_KEY_PATH").expect("JWT_PRIVATE_KEY_PATH must be set");
    let private_key = fs::read_to_string(private_key_path)?;

    let repo = create_token_repository(
        &config.database_url, config.database_min_connections, config.database_max_connections,
    ).await?;
    println!("Connected to database");

    println!("Generating {} tokens...", token_count);
    generate_and_insert_tokens(&repo, &private_key, &path, alg, &client_id, token_count).await?;

    println!("Done! Generated {} tokens", token_count);
    println!("Tokens written to {}", path);

    Ok(())
}

async fn generate_and_insert_tokens(
    repo: &dyn AccessTokenRepository,
    private_key: &str,
    tokens_path: &str,
    alg: Algorithm,
    client_id: &str,
    count: usize,
) -> Result<()> {
    let now = Utc::now();
    let timestamp = now.timestamp();

    let mut token_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(tokens_path)?;

    for i in 0..count {
        let jti = Uuid::new_v4().to_string();

        let claims = JWTClaims {
            jti: jti.clone(),
            sub: i.to_string(),
            aud: Some(client_id.to_string()),
            exp: (timestamp + 3600 * 24 * 365) as f64,
            iat: timestamp as f64,
            nbf: timestamp as f64,
            scopes: Some(vec!["openid".to_string()]),
            // Laravel doesn't encode these into user access token
            client_id: None,
            username: None,
            iss: None,
            user_id: None,
        };

        let token = encode(
            &Header::new(alg),
            &claims,
            &create_encoding_key(private_key, alg)?,
        )?;

        let now_naive = now.naive_utc();
        let meta = TokenMeta {
            id: jti,
            user_id: Some(i as i64),
            client_id: client_id.to_string(),
            name: None, // only for user generated tokens
            scopes: claims.scopes.as_ref().map(|s| s.join(" ")),
            revoked: false,
            created_at: now_naive,
            updated_at: now_naive,
            expires_at: (now + Duration::hours(1)).naive_utc(),
        };

        repo.add_token(meta).await?;

        writeln!(token_file, "{}", token)?;

        if (i + 1) % 100 == 0 {
            println!("Generated {} tokens", i + 1);
        }
    }

    Ok(())
}

fn create_encoding_key(private_key: &str, alg: Algorithm) -> Result<EncodingKey> {
    match alg {
        // RSA variants - use RSA public key
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 |
        Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => {
            EncodingKey::from_rsa_pem(private_key.as_bytes())
                .context("Invalid RSA private key format")
        }

        // ECDSA variants - use EC public key (same PEM format as RSA)
        Algorithm::ES256 | Algorithm::ES384 => {
            EncodingKey::from_ec_pem(private_key.as_bytes())
                .context("Invalid EC private key format")
        }

        // EdDSA variant
        Algorithm::EdDSA => {
            EncodingKey::from_ed_pem(private_key.as_bytes())
                .context("Invalid EdDSA private key format")
        }

        // HMAC variants - use symmetric secret
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            Ok(EncodingKey::from_secret(private_key.as_bytes()))
        }
    }
}

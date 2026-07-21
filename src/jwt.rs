use std::collections::HashSet;

use anyhow::{Context, Result, anyhow};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tracing::debug;

static DECODING_KEY: OnceCell<DecodingKey> = OnceCell::new();
static CONFIGURED_ALGORITHM: OnceCell<Algorithm> = OnceCell::new();
static VALIDATION: OnceCell<Validation> = OnceCell::new();

pub fn init_crypto(
    public_key: &str,
    alg: Algorithm,
    config_client_id: &Option<String>,
) -> Result<()> {
    let key = create_decoding_key(public_key, alg)?;

    DECODING_KEY
        .set(key)
        .map_err(|_| anyhow!("Crypto already initialized"))?;
    CONFIGURED_ALGORITHM
        .set(alg)
        .map_err(|_| anyhow!("Algorithm already initialized"))?;

    let validation = create_base_validation(alg, config_client_id);

    VALIDATION
        .set(validation)
        .map_err(|_| anyhow!("Validation already initialized"))?;

    debug!("Crypto initialized with algorithm: {:?}", alg);
    Ok(())
}

fn create_base_validation(alg: Algorithm, config_client_id: &Option<String>) -> Validation {
    let mut v = Validation::new(alg);

    if let Some(client_id) = config_client_id {
        let mut aud_set = HashSet::new();
        aud_set.insert(client_id.clone());
        v.aud = Some(aud_set);
        debug!("Using CLIENT_ID if there is no X-Client-Id header");
    } else {
        v.validate_aud = false;
        debug!(
            "No CLIENT_ID provided - skipping audience validation if there is no X-Client-Id header"
        );
    }

    v
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JWTClaims {
    pub jti: String,
    pub sub: String,
    pub aud: Option<String>,

    // ⚠️ Use f64 to handle both integers and floats
    // Laravel accepts int|float|string, we just need to parse it
    pub exp: f64,
    pub iat: f64,
    pub nbf: f64,

    #[serde(default, rename = "scopes")]
    pub scopes: Option<Vec<String>>,

    #[serde(default)]
    pub client_id: Option<String>,

    #[serde(default)]
    pub username: Option<String>,

    #[serde(default)]
    pub iss: Option<String>,

    #[serde(default)]
    pub user_id: Option<i64>,
}

// oauth_access_tokens table
#[derive(Debug, Clone)]
pub struct TokenMeta {
    pub id: String,
    pub user_id: Option<i64>,
    pub client_id: String,
    pub name: Option<String>,
    pub scopes: Option<String>,
    pub revoked: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub expires_at: chrono::NaiveDateTime,
}

pub fn validate_jwt(token: &str, client_id_from_header: Option<String>) -> Result<JWTClaims> {
    let decoding_key = DECODING_KEY.get().expect("Crypto not initialized");
    let expected_alg = *CONFIGURED_ALGORITHM
        .get()
        .expect("Algorithm not initialized");

    let header = jsonwebtoken::decode_header(token)
        .context("Invalid JWT format: failed to decode header")?;

    let token_alg = header.alg;
    if token_alg != expected_alg {
        debug!(
            "Algorithm mismatch: expected {:?}, got {:?}",
            expected_alg, token_alg
        );
        return Err(anyhow!("Algorithm mismatch"));
    }

    let custom_validation;
    let validation_ref = if client_id_from_header.is_some() {
        let mut v = Validation::new(expected_alg);

        if let Some(aud) = client_id_from_header {
            let mut aud_set = HashSet::new();
            aud_set.insert(aud);
            v.aud = Some(aud_set);
        } else {
            v.validate_aud = false;
        }
        custom_validation = v; // Move to outer scope
        &custom_validation
    } else {
        // reference to static, NO CLONE
        VALIDATION.get().expect("Validation not initialized")
    };

    let token_data =
        decode::<JWTClaims>(token, &decoding_key, validation_ref).map_err(|e| match e.kind() {
            ErrorKind::MissingRequiredClaim(claim) => {
                anyhow!("Missing required claim: {}", claim)
            }
            _ => {
                anyhow!("Token validation failed: {:#}", e)
            }
        })?;

    let claims = token_data.claims;

    Ok(claims)
}

fn create_decoding_key(public_key: &str, alg: Algorithm) -> Result<DecodingKey> {
    match alg {
        // RSA variants - use RSA public key
        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => DecodingKey::from_rsa_pem(public_key.as_bytes())
            .context("Invalid RSA public key format"),

        // ECDSA variants - use EC public key (same PEM format as RSA)
        Algorithm::ES256 | Algorithm::ES384 => {
            DecodingKey::from_ec_pem(public_key.as_bytes()).context("Invalid EC public key format")
        }

        // EdDSA variant
        Algorithm::EdDSA => DecodingKey::from_ed_pem(public_key.as_bytes())
            .context("Invalid EdDSA public key format"),

        // HMAC variants - use symmetric secret
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            Ok(DecodingKey::from_secret(public_key.as_bytes()))
        }
    }
}

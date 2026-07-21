use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use laravel_passport_introspection::jwt::JWTClaims;

pub struct AuthorizationServer {
    private_key: String,
}

impl AuthorizationServer {
    pub fn new(private_key: &str) -> Self {
        Self {
            private_key: private_key.to_string(),
        }
    }

    pub fn generate_user_access_token(&self, client_id: &str) -> (String, JWTClaims) {
        self.laravel_user_access_token(client_id, 3600)
    }

    pub fn generate_expired_user_access_token(&self, client_id: &str) -> (String, JWTClaims) {
        self.laravel_user_access_token(client_id, -3600)
    }

    fn laravel_user_access_token(&self, client_id: &str, expires_in_seconds: i64) -> (String, JWTClaims) {
        let now = Utc::now().timestamp_micros() as f64 / 1_000_000.0;
        let user_id = 12345;

        let claims = JWTClaims {
            jti: Uuid::new_v4().to_string(),
            sub: user_id.to_string(),
            aud: Some(client_id.to_string()),
            exp: now + expires_in_seconds as f64,
            iat: now,
            nbf: now,
            // default nuxt scope
            scopes: Some(vec!["openid".to_string()]),
            client_id: None,
            username: None,
            iss: None,
            user_id: None,
        };

        let token = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(self.private_key.as_bytes())
                .expect("Failed to parse RSA private key"),
        )
        .expect("Failed to encode JWT");

        debug!("Issued token: {}", token);

        (token, claims)
    }
}

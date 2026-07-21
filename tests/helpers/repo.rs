use anyhow::Result;
use chrono::{Utc, Duration};

use laravel_passport_introspection::{
    database::{
        AccessTokenRepository,
        fake::FakeAccessTokenRepository,
    },
    jwt::{JWTClaims, TokenMeta},
};

pub trait FakeRepoExt {
    async fn add_token_from_claims(&self, claims: &JWTClaims, revoked: bool) -> Result<()>;
}

impl FakeRepoExt for FakeAccessTokenRepository {
    async fn add_token_from_claims(&self, claims: &JWTClaims, revoked: bool) -> Result<()> {
        let now = Utc::now();
        let now_naive = now.naive_utc();

        let user_id = claims.sub.parse::<i64>().ok();

        self.add_token(TokenMeta {
            id: claims.jti.clone(),
            user_id,
            client_id: claims.aud.clone().unwrap(),
            name: None,
            scopes: claims.scopes.as_ref().map(|s| s.join(" ")),
            revoked,
            created_at: now_naive,
            updated_at: now_naive,
            expires_at: (now + Duration::hours(1)).naive_utc(),
        }).await.expect("Failed to add test token");

        Ok(())
    }
}

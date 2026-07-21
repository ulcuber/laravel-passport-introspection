use anyhow::Result;
use async_trait::async_trait;

use super::traits::AccessTokenRepository;
use super::mysql::MySqlAccessTokenRepository;
use super::postgres::PgAccessTokenRepository;
use super::fake::FakeAccessTokenRepository;

#[derive(Clone)]
pub enum AnyAccessTokenRepository {
    MySql(MySqlAccessTokenRepository),
    Postgres(PgAccessTokenRepository),
    Fake(FakeAccessTokenRepository),
}

#[async_trait]
impl AccessTokenRepository for AnyAccessTokenRepository {
    async fn is_token_revoked(&self, id: &str) -> Result<bool> {
        match self {
            AnyAccessTokenRepository::MySql(repo) => repo.is_token_revoked(id).await,
            AnyAccessTokenRepository::Postgres(repo) => repo.is_token_revoked(id).await,
            AnyAccessTokenRepository::Fake(repo) => repo.is_token_revoked(id).await,
        }
    }

    #[cfg(feature = "write-tokens")]
    async fn add_token(&self, meta: crate::jwt::TokenMeta) -> Result<()> {
        match self {
            AnyAccessTokenRepository::MySql(repo) => repo.add_token(meta).await,
            AnyAccessTokenRepository::Postgres(repo) => repo.add_token(meta).await,
            AnyAccessTokenRepository::Fake(repo) => repo.add_token(meta).await,
        }
    }
}

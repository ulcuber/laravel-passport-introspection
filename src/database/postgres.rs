use anyhow::{Context, Result};
use sqlx::{postgres::PgPool, Row};
use tracing::info;

use super::traits::AccessTokenRepository;

#[derive(Clone)]
pub struct PgAccessTokenRepository {
    pool: PgPool,
}

impl PgAccessTokenRepository {
    pub async fn new(database_url: &str, min_connections: u32, max_connections: u32) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections)
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl AccessTokenRepository for PgAccessTokenRepository {
    async fn is_token_revoked(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT revoked FROM oauth_access_tokens WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let revoked: bool = row.try_get("revoked")
                    .context("Failed to parse revoked column")?;
                Ok(revoked)
            }
            None => {
                info!("Token {} not found in database", id);
                Ok(true)
            }
        }
    }

    #[cfg(feature = "write-tokens")]
    async fn add_token(&self, meta: crate::jwt::TokenMeta) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO oauth_access_tokens
            (id, user_id, client_id, name, scopes, revoked, created_at, updated_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(meta.id)
        .bind(meta.user_id)
        .bind(meta.client_id)
        .bind(meta.name)
        .bind(meta.scopes)
        .bind(meta.revoked)
        .bind(meta.created_at)
        .bind(meta.updated_at)
        .bind(meta.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

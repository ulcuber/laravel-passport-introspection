use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{mysql::MySqlPool, Row};
use tracing::info;

use super::traits::AccessTokenRepository;

#[derive(Clone)]
pub struct MySqlAccessTokenRepository {
    pool: MySqlPool,
}

impl MySqlAccessTokenRepository {
    pub async fn new(database_url: &str, min_connections: u32, max_connections: u32) -> Result<Self> {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections)
            .max_lifetime(Duration::from_secs(300))
            .idle_timeout(Duration::from_secs(60))
            .connect(database_url)
            .await
            .context("Failed to connect to MySQL")?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl AccessTokenRepository for MySqlAccessTokenRepository {
    async fn is_token_revoked(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT revoked FROM oauth_access_tokens WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let revoked: i32 = row.try_get("revoked")
                    .context("Failed to parse revoked column")?;
                Ok(revoked == 1)
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

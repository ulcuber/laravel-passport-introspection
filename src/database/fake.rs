use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::debug;

use super::traits::AccessTokenRepository;
use crate::jwt::TokenMeta;

#[derive(Clone)]
pub struct FakeAccessTokenRepository {
    tokens: Arc<RwLock<HashMap<String, TokenMeta>>>,
}

impl FakeAccessTokenRepository {
    pub async fn new(
        _database_url: &str,
        _min_connections: u32,
        _max_connections: u32,
    ) -> Result<Self> {
        Ok(Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait::async_trait]
impl AccessTokenRepository for FakeAccessTokenRepository {
    async fn is_token_revoked(&self, id: &str) -> Result<bool> {
        let tokens = self.tokens.read().await;

        match tokens.get(id) {
            Some(meta) => {
                debug!("Token exists and is revoked");
                Ok(meta.revoked)
            }
            None => {
                debug!("Token doesn't exist and so is revoked");
                Ok(true)
            }
        }
    }

    #[cfg(feature = "write-tokens")]
    async fn add_token(&self, meta: TokenMeta) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(meta.id.clone(), meta);

        Ok(())
    }
}

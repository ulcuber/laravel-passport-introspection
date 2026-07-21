use anyhow::Result;

#[async_trait::async_trait]
pub trait AccessTokenRepository: Send + Sync + 'static {
    async fn is_token_revoked(&self, id: &str) -> Result<bool>;
    #[cfg(feature = "write-tokens")]
    async fn add_token(&self, meta: crate::jwt::TokenMeta) -> Result<()>;
}

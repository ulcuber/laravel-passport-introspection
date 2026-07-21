use anyhow::Result;

use super::any::AnyAccessTokenRepository;
use super::fake::FakeAccessTokenRepository;
use super::mysql::MySqlAccessTokenRepository;
use super::postgres::PgAccessTokenRepository;

pub enum DatabaseType {
    MySql,
    Postgres,
    Fake,
}

impl DatabaseType {
    pub fn from_url(url: &str) -> Result<Self> {
        if url.starts_with("mysql://") {
            Ok(Self::MySql)
        } else if url.starts_with("postgresql://") || url.starts_with("postgres://") {
            Ok(Self::Postgres)
        } else if url == "fake" {
            Ok(Self::Fake)
        } else {
            anyhow::bail!("Unsupported database URL: {}", url)
        }
    }
}

pub async fn create_token_repository(
    database_url: &str,
    min_connections: u32,
    max_connections: u32,
) -> Result<AnyAccessTokenRepository> {
    let db_type = DatabaseType::from_url(database_url)?;

    match db_type {
        DatabaseType::MySql => {
            let repo =
                MySqlAccessTokenRepository::new(database_url, min_connections, max_connections)
                    .await?;
            Ok(AnyAccessTokenRepository::MySql(repo))
        }
        DatabaseType::Postgres => {
            let repo = PgAccessTokenRepository::new(database_url, min_connections, max_connections)
                .await?;
            Ok(AnyAccessTokenRepository::Postgres(repo))
        }
        DatabaseType::Fake => {
            let repo =
                FakeAccessTokenRepository::new(database_url, min_connections, max_connections)
                    .await?;
            Ok(AnyAccessTokenRepository::Fake(repo))
        }
    }
}

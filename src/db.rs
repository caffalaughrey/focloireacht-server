use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::time::Duration;

#[derive(Clone)]
pub struct DbPools {
    pub lex: SqlitePool,
    pub term: SqlitePool,
}

impl DbPools {
    pub async fn connect(lex_dsn: &str, term_dsn: &str) -> Result<Self, sqlx::Error> {
        let lex = SqlitePoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect(lex_dsn)
            .await?;

        let term = SqlitePoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect(term_dsn)
            .await?;

        Ok(Self { lex, term })
    }
}



use axum::{extract::State, response::IntoResponse, Json};
use sqlx::Row;

use crate::{db::DbPools, models::meta::{MetaDbInfo, MetaResponse, SourceRow}};

pub async fn meta(State(pools): State<DbPools>) -> impl IntoResponse {
    let (lex, term) = tokio::join!(fetch_meta_for_db(&pools.lex), fetch_meta_for_db(&pools.term));

    let lex_info = lex.unwrap_or_else(|_| MetaDbInfo { schema_version: None, build_time: None, sources: vec![] });
    let term_info = term.unwrap_or_else(|_| MetaDbInfo { schema_version: None, build_time: None, sources: vec![] });

    Json(MetaResponse { lex: lex_info, term: term_info })
}

async fn fetch_meta_for_db(pool: &sqlx::SqlitePool) -> Result<MetaDbInfo, sqlx::Error> {
    let schema_version = sqlx::query("SELECT value FROM meta WHERE key = 'schema_version' LIMIT 1")
        .fetch_optional(pool)
        .await?
        .map(|row| row.get::<String, _>(0));

    let build_time = sqlx::query("SELECT value FROM meta WHERE key = 'build_time' LIMIT 1")
        .fetch_optional(pool)
        .await?
        .map(|row| row.get::<String, _>(0));

    let sources: Vec<SourceRow> = sqlx::query_as!(
        SourceRow,
        r#"SELECT id as "id!", name, version, license, url FROM sources"#
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(MetaDbInfo { schema_version, build_time, sources })
}



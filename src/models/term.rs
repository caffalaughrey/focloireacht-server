use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct TermRow {
    pub id: String,
    pub lang: String,
    pub term: String,
    pub pos: Option<String>,
    pub lookup_key: String,
    pub source_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TermSide { pub term_id: String, pub term: String, pub pos: Option<String>, pub domains: Vec<String>, pub source_id: String }

#[derive(Debug, Serialize, Clone)]
pub struct TermPairPayload { pub ga: TermSide, pub en: TermSide }

#[derive(Debug, Serialize)]
pub struct TermQueryResponse { pub query: QueryEcho, pub matches: Vec<TermPairPayload> }

#[derive(Debug, Serialize)]
pub struct QueryEcho { pub term: String, pub lang: String, pub domain: Option<String> }



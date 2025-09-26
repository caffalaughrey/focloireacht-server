use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MetaDbInfo {
    pub schema_version: Option<String>,
    pub build_time: Option<String>,
    pub sources: Vec<SourceRow>,
}

#[derive(Debug, Serialize)]
pub struct MetaResponse {
    pub lex: MetaDbInfo,
    pub term: MetaDbInfo,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct SourceRow {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub url: Option<String>,
}



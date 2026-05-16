use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use std::collections::HashMap;

use crate::{db::DbPools, models::lex::{EntryRow, SenseRow, ExampleRow, EtyRow, VariantRow, FreqRow, LexEntryPayload}};

const DEFAULT_LIMIT: i64 = 5;
const MAX_LIMIT: i64 = 50;

#[derive(Deserialize)]
pub struct LemmaQuery { pub lemma: String, pub limit: Option<i64> }

#[derive(Deserialize)]
pub struct VariantQuery { pub form: String, pub limit: Option<i64> }

fn sanitize_limit(limit: Option<i64>) -> Result<i64, (StatusCode, &'static str)> {
    let n = limit.unwrap_or(DEFAULT_LIMIT);
    if n <= 0 || n > MAX_LIMIT { return Err((StatusCode::BAD_REQUEST, "invalid limit")); }
    Ok(n)
}

fn validate_key(s: &str) -> Result<(), (StatusCode, &'static str)> {
    if s.is_empty() || s.len() > 128 { return Err((StatusCode::BAD_REQUEST, "invalid key")); }
    Ok(())
}

pub async fn get_by_lemma(State(pools): State<DbPools>, Query(q): Query<LemmaQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&q.lemma)?;
    let limit = sanitize_limit(q.limit)?;

    let entries: Vec<EntryRow> = sqlx::query_as!(
        EntryRow,
        r#"SELECT id as "id!", lemma as "lemma!", pos, sort_key, lookup_key, notes_raw FROM entries WHERE lookup_key = ? LIMIT ?"#,
        q.lemma,
        limit
    )
    .fetch_all(&pools.lex)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    Ok(Json(build_lex_payloads(&pools, entries).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?))
}

pub async fn get_by_variant(State(pools): State<DbPools>, Query(q): Query<VariantQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&q.form)?;
    let limit = sanitize_limit(q.limit)?;

    let entry_ids: Vec<(String,)> = sqlx::query_as(
        r#"SELECT entry_id FROM variants WHERE form = ? LIMIT ?"#
    )
    .bind(&q.form)
    .bind(limit)
    .fetch_all(&pools.lex)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    let ids: Vec<String> = entry_ids.into_iter().map(|t| t.0).collect();
    let entries: Vec<EntryRow> = if ids.is_empty() { vec![] } else {
        let ids_json = serde_json::to_string(&ids).unwrap();
        sqlx::query_as!(
            EntryRow,
            r#"SELECT id as "id!", lemma as "lemma!", pos, sort_key, lookup_key, notes_raw FROM entries WHERE id IN (SELECT value FROM json_each(?))"#,
            ids_json
        ).fetch_all(&pools.lex).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    };
    Ok(Json(build_lex_payloads(&pools, entries).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?))
}

#[derive(Deserialize)]
pub struct LexBatchBody { pub lemmas: Option<Vec<String>>, pub variants: Option<Vec<String>>, pub limit: Option<i64> }

#[derive(serde::Serialize)]
pub struct LexBatchResponse { pub lemmas: HashMap<String, Vec<LexEntryPayload>>, pub variants: HashMap<String, Vec<LexEntryPayload>> }

pub async fn post_batch(State(pools): State<DbPools>, axum::Json(body): axum::Json<LexBatchBody>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let limit = sanitize_limit(body.limit)?;
    let mut lemma_map: HashMap<String, Vec<LexEntryPayload>> = HashMap::new();
    let mut variant_map: HashMap<String, Vec<LexEntryPayload>> = HashMap::new();

    if let Some(lemmas) = body.lemmas.clone() {
        for lemma in lemmas.iter() {
            validate_key(lemma)?;
            let entries: Vec<EntryRow> = sqlx::query_as!(
                EntryRow,
                r#"SELECT id as "id!", lemma as "lemma!", pos, sort_key, lookup_key, notes_raw FROM entries WHERE lookup_key = ? LIMIT ?"#,
                lemma,
                limit
            )
            .fetch_all(&pools.lex)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
            let payloads = build_lex_payloads(&pools, entries).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
            lemma_map.insert(lemma.clone(), payloads);
        }
    }

    if let Some(variants) = body.variants.clone() {
        for form in variants.iter() {
            validate_key(form)?;
            let entry_ids: Vec<(String,)> = sqlx::query_as(
                r#"SELECT entry_id FROM variants WHERE form = ? LIMIT ?"#
            )
            .bind(form)
            .bind(limit)
            .fetch_all(&pools.lex)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
            let ids: Vec<String> = entry_ids.into_iter().map(|t| t.0).collect();
            let entries: Vec<EntryRow> = if ids.is_empty() { vec![] } else {
                let ids_json = serde_json::to_string(&ids).unwrap();
                sqlx::query_as!(
                    EntryRow,
                    r#"SELECT id as "id!", lemma as "lemma!", pos, sort_key, lookup_key, notes_raw FROM entries WHERE id IN (SELECT value FROM json_each(?))"#,
                    ids_json
                ).fetch_all(&pools.lex).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
            };
            let payloads = build_lex_payloads(&pools, entries).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
            variant_map.insert(form.clone(), payloads);
        }
    }

    Ok(Json(LexBatchResponse { lemmas: lemma_map, variants: variant_map }))
}

// Legacy MCP-style lookup endpoint: {"q":"madra"} -> returns entries for lemma lookup_key
#[derive(Deserialize)]
pub struct LegacyLookupBody { pub q: String, pub limit: Option<i64> }

pub async fn lookup_legacy(State(pools): State<DbPools>, Json(body): Json<LegacyLookupBody>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&body.q)?;
    let limit = sanitize_limit(body.limit)?;
    let entries: Vec<EntryRow> = sqlx::query_as!(
        EntryRow,
        r#"SELECT id as "id!", lemma as "lemma!", pos, sort_key, lookup_key, notes_raw FROM entries WHERE lookup_key = ? LIMIT ?"#,
        body.q,
        limit
    )
    .fetch_all(&pools.lex)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    let payloads = build_lex_payloads(&pools, entries).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    Ok(Json(payloads))
}

async fn build_lex_payloads(pools: &DbPools, entries: Vec<EntryRow>) -> Result<Vec<LexEntryPayload>, sqlx::Error> {
    if entries.is_empty() { return Ok(vec![]); }
    let entry_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    let id_json = serde_json::to_string(&entry_ids).unwrap();

    let senses: Vec<SenseRow> = sqlx::query_as!(
        SenseRow,
        r#"SELECT id as "id!", entry_id as "entry_id!", gloss, usage_labels, notes_raw, source_id as "source_id!" FROM senses WHERE entry_id IN (SELECT value FROM json_each(?))"#,
        id_json
    ).fetch_all(&pools.lex).await?;

    let sense_ids: Vec<String> = senses.iter().map(|s| s.id.clone()).collect();
    let examples: Vec<ExampleRow> = if sense_ids.is_empty() { vec![] } else {
        let sense_json = serde_json::to_string(&sense_ids).unwrap();
        sqlx::query_as!(
            ExampleRow,
            r#"SELECT id as "id!", sense_id as "sense_id!", text as "text!", trans, source_id as "source_id!" FROM examples WHERE sense_id IN (SELECT value FROM json_each(?))"#,
            sense_json
        ).fetch_all(&pools.lex).await?
    };

    let etymology: Vec<EtyRow> = sqlx::query_as!(
        EtyRow,
        r#"SELECT id as "id!", entry_id as "entry_id!", src_lang, value, ref as "ref_?", source_id as "source_id!" FROM etymology WHERE entry_id IN (SELECT value FROM json_each(?))"#,
        id_json
    ).fetch_all(&pools.lex).await?;

    let variants: Vec<VariantRow> = sqlx::query_as!(
        VariantRow,
        r#"SELECT id as "id!", entry_id as "entry_id!", form as "form!", vtype, dialect, note, source_id as "source_id!" FROM variants WHERE entry_id IN (SELECT value FROM json_each(?))"#,
        id_json
    ).fetch_all(&pools.lex).await?;

    let freq: Vec<FreqRow> = sqlx::query_as!(
        FreqRow,
        r#"SELECT entry_id as "entry_id!", lemma as "lemma!", rank as "rank?", count_tokens as "count_tokens?", per_million as "per_million?", note, source_id as "source_id!" FROM freq WHERE entry_id IN (SELECT value FROM json_each(?))"#,
        id_json
    ).fetch_all(&pools.lex).await?;

    let mut senses_by_entry: HashMap<String, Vec<SenseRow>> = HashMap::new();
    for s in senses { senses_by_entry.entry(s.entry_id.clone()).or_default().push(s); }
    let mut ex_by_sense: HashMap<String, Vec<ExampleRow>> = HashMap::new();
    for ex in examples { ex_by_sense.entry(ex.sense_id.clone()).or_default().push(ex); }
    let mut ety_by_entry: HashMap<String, Vec<EtyRow>> = HashMap::new();
    for e in etymology { ety_by_entry.entry(e.entry_id.clone()).or_default().push(e); }
    let mut var_by_entry: HashMap<String, Vec<VariantRow>> = HashMap::new();
    for v in variants { var_by_entry.entry(v.entry_id.clone()).or_default().push(v); }
    let mut freq_by_entry: HashMap<String, FreqRow> = HashMap::new();
    for f in freq { freq_by_entry.insert(f.entry_id.clone(), f); }

    let mut result = Vec::with_capacity(entries.len());
    for entry in entries.into_iter() {
        let senses_vec = senses_by_entry.remove(&entry.id).unwrap_or_default();
        let mut examples_vec = Vec::new();
        for s in &senses_vec {
            if let Some(list) = ex_by_sense.get(&s.id) { examples_vec.extend(list.clone()); }
        }
        let ety_vec = ety_by_entry.remove(&entry.id).unwrap_or_default();
        let var_vec = var_by_entry.remove(&entry.id).unwrap_or_default();
        let freq_opt = freq_by_entry.remove(&entry.id);
        result.push(LexEntryPayload { entry, senses: senses_vec, examples: examples_vec, etymology: ety_vec, variants: var_vec, freq: freq_opt });
    }
    Ok(result)
}



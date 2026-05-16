use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use std::collections::HashMap;

use crate::{db::DbPools, models::term::{TermRow, TermSide, TermPairPayload, TermQueryResponse, QueryEcho}};

const DEFAULT_LIMIT: i64 = 5;
const MAX_LIMIT: i64 = 50;

fn sanitize_limit(limit: Option<i64>) -> Result<i64, (StatusCode, &'static str)> {
    let n = limit.unwrap_or(DEFAULT_LIMIT);
    if n <= 0 || n > MAX_LIMIT { return Err((StatusCode::BAD_REQUEST, "invalid limit")); }
    Ok(n)
}

fn validate_key(s: &str) -> Result<(), (StatusCode, &'static str)> {
    if s.is_empty() || s.len() > 128 { return Err((StatusCode::BAD_REQUEST, "invalid key")); }
    Ok(())
}

#[derive(Deserialize)]
pub struct TermQuery { pub term: String, pub domain: Option<String>, pub limit: Option<i64> }

pub async fn en2ga(State(pools): State<DbPools>, Query(q): Query<TermQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    direction(&pools, "en", &q.term, q.domain.clone(), q.limit).await
        .map(|matches| Json(TermQueryResponse { query: QueryEcho { term: q.term, lang: "en".to_string(), domain: q.domain }, matches }))
}

pub async fn ga2en(State(pools): State<DbPools>, Query(q): Query<TermQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    direction(&pools, "ga", &q.term, q.domain.clone(), q.limit).await
        .map(|matches| Json(TermQueryResponse { query: QueryEcho { term: q.term, lang: "ga".to_string(), domain: q.domain }, matches }))
}

async fn direction(pools: &DbPools, source_lang: &str, term: &str, domain: Option<String>, limit: Option<i64>) -> Result<Vec<TermPairPayload>, (StatusCode, &'static str)> {
    validate_key(term)?;
    if let Some(ref d) = domain { validate_key(d)?; }
    let limit = sanitize_limit(limit)?;

    // source terms
    let source_terms: Vec<TermRow> = sqlx::query_as!(
        TermRow,
        r#"SELECT id as "id!", lang as "lang!", term as "term!", pos, lookup_key as "lookup_key!", source_id as "source_id!" FROM terms WHERE lang = ? AND lookup_key = ? LIMIT ?"#,
        source_lang,
        term,
        limit
    ).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    if source_terms.is_empty() { return Ok(vec![]); }

    let src_ids: Vec<String> = source_terms.iter().map(|t| t.id.clone()).collect();
    let src_ids_json = serde_json::to_string(&src_ids).unwrap();

    let pairs: Vec<(String, String)> = if source_lang == "en" {
        sqlx::query_as(
            r#"SELECT p.ga_term_id, p.en_term_id FROM term_pairs p WHERE p.en_term_id IN (SELECT value FROM json_each(?))"#
        ).bind(&src_ids_json).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
            .into_iter().map(|row: (String, String)| row).collect()
    } else {
        sqlx::query_as(
            r#"SELECT p.en_term_id, p.ga_term_id FROM term_pairs p WHERE p.ga_term_id IN (SELECT value FROM json_each(?))"#
        ).bind(&src_ids_json).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
            .into_iter().map(|row: (String, String)| row).collect()
    };

    if pairs.is_empty() { return Ok(vec![]); }

    let (first_ids, second_ids): (Vec<String>, Vec<String>) = pairs.iter().cloned().unzip();
    let target_ids: Vec<String> = first_ids;
    let src_ids_again: Vec<String> = second_ids;

    let target_json = serde_json::to_string(&target_ids).unwrap();
    let src_json = serde_json::to_string(&src_ids_again).unwrap();

    let target_terms: Vec<TermRow> = sqlx::query_as!(
        TermRow,
        r#"SELECT id as "id!", lang as "lang!", term as "term!", pos, lookup_key as "lookup_key!", source_id as "source_id!" FROM terms WHERE id IN (SELECT value FROM json_each(?))"#,
        target_json
    ).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    let mut domains_by_term: HashMap<String, Vec<String>> = HashMap::new();
    if !target_terms.is_empty() {
        let drows: Vec<(String, String)> = if let Some(ref dlabel) = domain {
            sqlx::query_as(
                r#"SELECT l.term_id, d.label
                   FROM term_domain_links l
                   JOIN term_domains d ON d.id = l.domain_id
                  WHERE l.term_id IN (SELECT value FROM json_each(?))
                    AND (d.label = ? OR lower(trim(d.label)) = lower(trim(?)))"#
            ).bind(&target_json).bind(dlabel).bind(dlabel)
             .fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
        } else {
            sqlx::query_as(
                r#"SELECT l.term_id, d.label
                   FROM term_domain_links l
                   JOIN term_domains d ON d.id = l.domain_id
                  WHERE l.term_id IN (SELECT value FROM json_each(?))"#
            ).bind(&target_json)
             .fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
        };
        for (tid, label) in drows { domains_by_term.entry(tid).or_default().push(label); }
    }

    let src_terms: Vec<TermRow> = sqlx::query_as!(
        TermRow,
        r#"SELECT id as "id!", lang as "lang!", term as "term!", pos, lookup_key as "lookup_key!", source_id as "source_id!" FROM terms WHERE id IN (SELECT value FROM json_each(?))"#,
        src_json
    ).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    let mut src_map: HashMap<String, TermRow> = HashMap::new();
    for s in src_terms { src_map.insert(s.id.clone(), s); }
    let mut tgt_map: HashMap<String, TermRow> = HashMap::new();
    for t in &target_terms { tgt_map.insert(t.id.clone(), t.clone()); }

    let mut out = Vec::new();
    for (tgt, src) in pairs.into_iter() {
        if let (Some(t), Some(s)) = (tgt_map.get(&tgt), src_map.get(&src)) {
            let (ga, en) = if t.lang == "ga" { (t, s) } else { (s, t) };
            // if a domain filter was provided, ensure target has at least one label
            if let Some(ref dlabel) = domain {
                if domains_by_term.get(&ga.id).map(|v| v.iter().any(|x| x == dlabel || x.to_lowercase().trim() == dlabel.to_lowercase().trim())).unwrap_or(false) == false {
                    continue;
                }
            }
            let ga_side = TermSide { term_id: ga.id.clone(), term: ga.term.clone(), pos: ga.pos.clone(), domains: domains_by_term.remove(&ga.id).unwrap_or_default(), source_id: ga.source_id.clone() };
            let en_side = TermSide { term_id: en.id.clone(), term: en.term.clone(), pos: en.pos.clone(), domains: vec![], source_id: en.source_id.clone() };
            out.push(TermPairPayload { ga: ga_side, en: en_side });
        }
    }

    Ok(out)
}

#[derive(serde::Serialize)]
pub struct DomainsResponse { pub domains: Vec<DomainCountRow> }

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DomainCountRow { pub label: String, pub count: i64 }

pub async fn domains(State(pools): State<DbPools>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let rows: Vec<DomainCountRow> = sqlx::query_as!(
        DomainCountRow,
        r#"SELECT d.label as "label!", COUNT(*) as "count!"
           FROM term_domain_links l
           JOIN term_domains d ON d.id = l.domain_id
          GROUP BY d.label
          ORDER BY d.label"#
    ).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    Ok(Json(DomainsResponse { domains: rows }))
}

// --- Domain search ---

#[derive(Deserialize)]
pub struct DomainSearchQuery { pub q: String, pub limit: Option<i64> }

#[derive(serde::Serialize)]
pub struct DomainSearchResponse { pub domains: Vec<DomainSearchRow> }

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DomainSearchRow { pub label: String, pub count: i64 }

pub async fn domains_search(State(pools): State<DbPools>, Query(q): Query<DomainSearchQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&q.q)?;
    let limit = q.limit.unwrap_or(20).min(100);
    // Normalise query so callers can use & or &amp; interchangeably
    let q_norm = q.q.replace("&amp;", "&");
    let pattern = format!("%{}%", q_norm);
    let rows: Vec<DomainSearchRow> = sqlx::query_as(
        r#"SELECT REPLACE(d.label, '&amp;', '&') as label, COUNT(*) as count
           FROM term_domains d
           JOIN term_domain_links l ON l.domain_id = d.id
           WHERE REPLACE(d.label, '&amp;', '&') LIKE ? AND CAST(d.label AS INTEGER) = 0
           GROUP BY d.label
           ORDER BY count DESC
           LIMIT ?"#
    ).bind(&pattern).bind(limit).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    Ok(Json(DomainSearchResponse { domains: rows }))
}

// --- Domain vocab ---

#[derive(Deserialize)]
pub struct VocabQuery { pub domain: String, pub lang: Option<String>, pub limit: Option<i64> }

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct VocabEntry { pub ga: String, pub en: String, pub pos: Option<String> }

#[derive(serde::Serialize)]
pub struct VocabResponse { pub domain: String, pub lang: String, pub count: usize, pub terms: Vec<VocabEntry> }

pub async fn vocab(State(pools): State<DbPools>, Query(q): Query<VocabQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&q.domain)?;
    let limit = q.limit.unwrap_or(200).min(500);
    let lang = q.lang.as_deref().unwrap_or("ga");
    if lang != "ga" && lang != "en" { return Err((StatusCode::BAD_REQUEST, "invalid lang")); }

    // Normalise HTML entities on both sides so callers can use plain & or &amp;
    let domain_norm = q.domain.replace("&amp;", "&");
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        r#"SELECT t_ga.term, t_en.term, t_ga.pos
           FROM term_pairs p
           JOIN terms t_ga ON t_ga.id = p.ga_term_id
           JOIN terms t_en ON t_en.id = p.en_term_id
           JOIN term_domain_links l ON l.term_id = t_ga.id
           JOIN term_domains d ON d.id = l.domain_id
           WHERE REPLACE(d.label, '&amp;', '&') = ?
           ORDER BY t_ga.term
           LIMIT ?"#
    ).bind(&domain_norm).bind(limit).fetch_all(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    let terms: Vec<VocabEntry> = rows.into_iter().map(|(ga, en, pos)| VocabEntry { ga, en, pos }).collect();
    let count = terms.len();
    Ok(Json(VocabResponse { domain: q.domain, lang: lang.to_string(), count, terms }))
}

// --- Validate ---

#[derive(Deserialize)]
pub struct ValidateQuery { pub term: String, pub lang: String, pub domain: Option<String> }

#[derive(serde::Serialize)]
pub struct ValidateResponse { pub valid: bool }

pub async fn validate(State(pools): State<DbPools>, Query(q): Query<ValidateQuery>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    validate_key(&q.term)?; if let Some(ref d) = q.domain { validate_key(d)?; }
    let lang = q.lang.as_str(); if lang != "ga" && lang != "en" { return Err((StatusCode::BAD_REQUEST, "invalid lang")); }

    let src: Option<(String,)> = sqlx::query_as(
        r#"SELECT id FROM terms WHERE lang = ? AND lookup_key = ? LIMIT 1"#
    ).bind(lang).bind(&q.term).fetch_optional(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    let Some((src_id,)) = src else { return Ok(Json(ValidateResponse { valid: false })); };

    let pair_exists: Option<(String, String)> = if lang == "en" {
        sqlx::query_as(r#"SELECT ga_term_id, en_term_id FROM term_pairs WHERE en_term_id = ? LIMIT 1"#)
            .bind(&src_id).fetch_optional(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    } else {
        sqlx::query_as(r#"SELECT en_term_id, ga_term_id FROM term_pairs WHERE ga_term_id = ? LIMIT 1"#)
            .bind(&src_id).fetch_optional(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    };
    if pair_exists.is_none() { return Ok(Json(ValidateResponse { valid: false })); }

    if let Some(ref dlabel) = q.domain {
        let target_id: String = if lang == "en" { pair_exists.as_ref().unwrap().0.clone() } else { pair_exists.as_ref().unwrap().0.clone() };
        let exists: Option<(i64,)> = sqlx::query_as(
            r#"SELECT 1 FROM term_domain_links l JOIN term_domains d ON d.id = l.domain_id WHERE l.term_id = ? AND (d.label = ? OR lower(trim(d.label)) = lower(trim(?))) LIMIT 1"#
        ).bind(&target_id).bind(dlabel).bind(dlabel).fetch_optional(&pools.term).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
        return Ok(Json(ValidateResponse { valid: exists.is_some() }));
    }

    Ok(Json(ValidateResponse { valid: true }))
}

#[derive(Deserialize)]
pub struct BatchBody { pub lang: String, pub terms: Vec<String>, pub domain: Option<String>, pub limit: Option<i64> }

pub async fn batch(State(pools): State<DbPools>, Json(body): Json<BatchBody>) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let lang = body.lang.as_str(); if lang != "ga" && lang != "en" { return Err((StatusCode::BAD_REQUEST, "invalid lang")); }
    let mut results = Vec::with_capacity(body.terms.len());
    for term in &body.terms {
        let matches = direction(&pools, lang, term, body.domain.clone(), body.limit).await?;
        results.push(TermQueryResponse { query: QueryEcho { term: term.clone(), lang: lang.to_string(), domain: body.domain.clone() }, matches });
    }
    Ok(Json(results))
}



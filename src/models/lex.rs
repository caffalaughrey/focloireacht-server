use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct EntryRow {
    pub id: String,
    pub lemma: String,
    pub pos: Option<String>,
    pub sort_key: Option<String>,
    pub lookup_key: Option<String>,
    pub notes_raw: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct SenseRow {
    pub id: String,
    pub entry_id: String,
    pub gloss: Option<String>,
    pub usage_labels: Option<String>,
    pub notes_raw: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct ExampleRow {
    pub id: String,
    pub sense_id: String,
    pub text: String,
    pub trans: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct EtyRow {
    pub id: String,
    pub entry_id: String,
    pub src_lang: Option<String>,
    pub value: Option<String>,
    pub ref_: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct VariantRow {
    pub id: String,
    pub entry_id: String,
    pub form: String,
    pub vtype: Option<String>,
    pub dialect: Option<String>,
    pub note: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct FreqRow {
    pub entry_id: String,
    pub lemma: String,
    pub rank: Option<i64>,
    pub count_tokens: Option<i64>,
    pub per_million: Option<f64>,
    pub note: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LexEntryPayload {
    pub entry: EntryRow,
    pub senses: Vec<SenseRow>,
    pub examples: Vec<ExampleRow>,
    pub etymology: Vec<EtyRow>,
    pub variants: Vec<VariantRow>,
    pub freq: Option<FreqRow>,
}



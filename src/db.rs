use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Row, SqlitePool};
use std::path::Path;

pub type DbPool = SqlitePool;

pub const MANUAL_SOURCE_ID: &str = "00000000-0000-0000-0000-000000000000";

pub async fn open_and_migrate(db_path: &Path) -> Result<DbPool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    seed_manual_source(&pool).await?;
    Ok(pool)
}

async fn seed_manual_source(pool: &DbPool) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO sources (id, source_path, title, source_type, content_hash, ingested_at) \
         VALUES (?, 'manual', 'Manual edges', 'manual', 'manual', ?)"
    )
    .bind(MANUAL_SOURCE_ID)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub fn match_key(name: &str, entity_type: &str) -> String {
    let normalized: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let slug = normalized.split_whitespace().collect::<Vec<_>>().join("-");
    format!("{entity_type}:{slug}")
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SourceRecord {
    pub id: String,
    pub source_path: String,
    pub title: Option<String>,
    pub source_type: String,
    pub content_hash: String,
    pub toc_hash: Option<String>,
    pub preprocessed_toc: i64,
    pub toc_author: Option<String>,
    pub toc_generated_at: Option<String>,
    pub ingested_at: String,
    pub toc_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NoteRecord {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub match_key: String,
    pub file_path: String,
    pub summary_1: Option<String>,
    pub summary_5: Option<String>,
    pub merge_category: String,
    pub created_from: String,
    pub source_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SourceContributionRecord {
    pub id: String,
    pub source_id: String,
    pub note_id: String,
    pub toc_address: Option<String>,
    pub hint: Option<String>,
    pub contribution_type: String,
    pub payload: Option<String>,
    pub contributed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EdgeRecord {
    pub id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub edge_type: String,
    pub why: Option<String>,
    pub from_source: String,
    pub weight: f64,
    pub metadata: Option<String>,
    pub created_at: String,
}

pub async fn insert_source(pool: &DbPool, rec: &SourceRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO sources \
         (id, source_path, title, source_type, content_hash, toc_hash, \
          preprocessed_toc, toc_author, toc_generated_at, ingested_at, toc_text) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&rec.id)
    .bind(&rec.source_path)
    .bind(&rec.title)
    .bind(&rec.source_type)
    .bind(&rec.content_hash)
    .bind(&rec.toc_hash)
    .bind(rec.preprocessed_toc)
    .bind(&rec.toc_author)
    .bind(&rec.toc_generated_at)
    .bind(&rec.ingested_at)
    .bind(&rec.toc_text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_source(pool: &DbPool, id: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

pub async fn find_source_by_content_hash(pool: &DbPool, hash: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE content_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

pub async fn find_source_by_path(pool: &DbPool, source_path: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE source_path = ? ORDER BY ingested_at DESC LIMIT 1")
        .bind(source_path)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

fn row_to_source(row: sqlx::sqlite::SqliteRow) -> SourceRecord {
    SourceRecord {
        id: row.get("id"),
        source_path: row.get("source_path"),
        title: row.get("title"),
        source_type: row.get("source_type"),
        content_hash: row.get("content_hash"),
        toc_hash: row.get("toc_hash"),
        preprocessed_toc: row.get("preprocessed_toc"),
        toc_author: row.get("toc_author"),
        toc_generated_at: row.get("toc_generated_at"),
        ingested_at: row.get("ingested_at"),
        toc_text: row.get("toc_text"),
    }
}

pub async fn insert_note(pool: &DbPool, rec: &NoteRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO notes \
         (id, entity_type, name, match_key, file_path, summary_1, summary_5, \
          merge_category, created_from, source_count, created_at, updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?) \
         ON CONFLICT(match_key) DO UPDATE SET \
           summary_1 = excluded.summary_1, \
           summary_5 = excluded.summary_5, \
           updated_at = excluded.updated_at",
    )
    .bind(&rec.id)
    .bind(&rec.entity_type)
    .bind(&rec.name)
    .bind(&rec.match_key)
    .bind(&rec.file_path)
    .bind(&rec.summary_1)
    .bind(&rec.summary_5)
    .bind(&rec.merge_category)
    .bind(&rec.created_from)
    .bind(rec.source_count)
    .bind(&rec.created_at)
    .bind(&rec.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_note(pool: &DbPool, id: &str) -> Result<Option<NoteRecord>> {
    let row = sqlx::query("SELECT * FROM notes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_note))
}

pub async fn find_note_by_match_key(pool: &DbPool, key: &str) -> Result<Option<NoteRecord>> {
    let row = sqlx::query("SELECT * FROM notes WHERE match_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_note))
}

fn row_to_note(row: sqlx::sqlite::SqliteRow) -> NoteRecord {
    NoteRecord {
        id: row.get("id"),
        entity_type: row.get("entity_type"),
        name: row.get("name"),
        match_key: row.get("match_key"),
        file_path: row.get("file_path"),
        summary_1: row.get("summary_1"),
        summary_5: row.get("summary_5"),
        merge_category: row.get("merge_category"),
        created_from: row.get("created_from"),
        source_count: row.get("source_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn update_note_file_path(pool: &DbPool, id: &str, path: &str) -> Result<()> {
    let now = now_rfc3339();
    sqlx::query("UPDATE notes SET file_path = ?, updated_at = ? WHERE id = ?")
        .bind(path)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_source_count(pool: &DbPool, note_id: &str) -> Result<()> {
    let now = now_rfc3339();
    sqlx::query("UPDATE notes SET source_count = source_count + 1, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(note_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_contribution(pool: &DbPool, rec: &SourceContributionRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO source_contributions \
         (id, source_id, note_id, toc_address, hint, contribution_type, payload, contributed_at) \
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(&rec.id)
    .bind(&rec.source_id)
    .bind(&rec.note_id)
    .bind(&rec.toc_address)
    .bind(&rec.hint)
    .bind(&rec.contribution_type)
    .bind(&rec.payload)
    .bind(&rec.contributed_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_edge_if_not_exists(pool: &DbPool, rec: &EdgeRecord) -> Result<bool> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO edges \
         (id, source_note_id, target_note_id, edge_type, why, from_source, weight, metadata, created_at) \
         VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(&rec.id)
    .bind(&rec.source_note_id)
    .bind(&rec.target_note_id)
    .bind(&rec.edge_type)
    .bind(&rec.why)
    .bind(&rec.from_source)
    .bind(rec.weight)
    .bind(&rec.metadata)
    .bind(&rec.created_at)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn edges_for_note(pool: &DbPool, note_id: &str) -> Result<Vec<EdgeRecord>> {
    let rows = sqlx::query(
        "SELECT * FROM edges WHERE source_note_id = ? OR target_note_id = ?",
    )
    .bind(note_id)
    .bind(note_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EdgeRecord {
            id: row.get("id"),
            source_note_id: row.get("source_note_id"),
            target_note_id: row.get("target_note_id"),
            edge_type: row.get("edge_type"),
            why: row.get("why"),
            from_source: row.get("from_source"),
            weight: row.get("weight"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
        })
        .collect())
}

pub async fn search_notes(pool: &DbPool, query: &str, limit: i64) -> Result<Vec<NoteRecord>> {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let rows = sqlx::query(
        "SELECT * FROM notes \
         WHERE name LIKE ? ESCAPE '\\' OR summary_1 LIKE ? ESCAPE '\\' OR summary_5 LIKE ? ESCAPE '\\' \
         ORDER BY updated_at DESC \
         LIMIT ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| NoteRecord {
        id: row.get("id"),
        entity_type: row.get("entity_type"),
        name: row.get("name"),
        match_key: row.get("match_key"),
        file_path: row.get("file_path"),
        summary_1: row.get("summary_1"),
        summary_5: row.get("summary_5"),
        merge_category: row.get("merge_category"),
        created_from: row.get("created_from"),
        source_count: row.get("source_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }).collect())
}

pub async fn find_contribution_by_source_toc(
    pool: &DbPool,
    source_id: &str,
    toc_address: &str,
) -> Result<Option<(SourceContributionRecord, NoteRecord)>> {
    let row = sqlx::query(
        "SELECT sc.id as sc_id, sc.source_id, sc.note_id, sc.toc_address, sc.hint, \
         sc.contribution_type, sc.payload, sc.contributed_at, \
         n.id as n_id, n.entity_type, n.name, n.match_key, n.file_path, \
         n.summary_1, n.summary_5, n.merge_category, n.created_from, \
         n.source_count, n.created_at, n.updated_at \
         FROM source_contributions sc \
         JOIN notes n ON n.id = sc.note_id \
         WHERE sc.source_id = ? AND sc.toc_address = ?",
    )
    .bind(source_id)
    .bind(toc_address)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        let contrib = SourceContributionRecord {
            id: r.get("sc_id"),
            source_id: r.get("source_id"),
            note_id: r.get("note_id"),
            toc_address: r.get("toc_address"),
            hint: r.get("hint"),
            contribution_type: r.get("contribution_type"),
            payload: r.get("payload"),
            contributed_at: r.get("contributed_at"),
        };
        let note = NoteRecord {
            id: r.get("n_id"),
            entity_type: r.get("entity_type"),
            name: r.get("name"),
            match_key: r.get("match_key"),
            file_path: r.get("file_path"),
            summary_1: r.get("summary_1"),
            summary_5: r.get("summary_5"),
            merge_category: r.get("merge_category"),
            created_from: r.get("created_from"),
            source_count: r.get("source_count"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        };
        (contrib, note)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_key_canonical_forms() {
        assert_eq!(match_key("Ian Kitajima", "person"), "person:ian-kitajima");
        assert_eq!(match_key("PICHTR", "organization"), "organization:pichtr");
        assert_eq!(match_key("Sovereign AI", "concept"), "concept:sovereign-ai");
    }

    #[test]
    fn match_key_special_chars() {
        assert_eq!(match_key("O'Brien & Co.", "organization"), "organization:o-brien-co");
    }

    #[test]
    fn match_key_leading_trailing_spaces() {
        assert_eq!(match_key("  some note  ", "note"), "note:some-note");
    }
}

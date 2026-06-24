use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

pub type DbPool = PgPool;

pub const MANUAL_SOURCE_ID: &str = "00000000-0000-0000-0000-000000000000";

pub async fn connect_and_migrate(database_url: &str) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
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

/// Fixed-precision rfc3339 (6-digit microseconds, `+00:00` offset) for
/// `last_accessed_at`. Unlike `now_rfc3339()` (chrono AutoSi → variable 0/3/6/9
/// fractional digits), this matches the migration's backfill format exactly, so
/// values lexically sort in true chronological order — required for the crawl's
/// coldest-first ordering and (later) eviction.
pub fn now_rfc3339_micros() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
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
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NoteRecord {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub match_key: String,
    pub lede: Option<String>,
    pub why: Option<String>,
    pub content: Option<String>,
    pub has_conflicts: i64,
    pub conflicts_updated_at: Option<String>,
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
          preprocessed_toc, toc_author, toc_generated_at, ingested_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
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
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_source(pool: &DbPool, id: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

pub async fn find_source_by_content_hash(pool: &DbPool, hash: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE content_hash = $1")
        .bind(hash)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

pub async fn find_source_by_path(pool: &DbPool, source_path: &str) -> Result<Option<SourceRecord>> {
    let row = sqlx::query("SELECT * FROM sources WHERE source_path = $1 ORDER BY ingested_at DESC LIMIT 1")
        .bind(source_path)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_source))
}

fn row_to_source(row: sqlx::postgres::PgRow) -> SourceRecord {
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
    }
}

pub async fn insert_note(pool: &DbPool, rec: &NoteRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO notes \
         (id, entity_type, name, match_key, lede, why, content, \
          has_conflicts, conflicts_updated_at, merge_category, created_from, \
          source_count, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) \
         ON CONFLICT(match_key) DO UPDATE SET \
           lede = COALESCE(notes.lede, excluded.lede), \
           why = COALESCE(notes.why, excluded.why), \
           content = COALESCE(notes.content, excluded.content), \
           source_count = notes.source_count + 1, \
           updated_at = excluded.updated_at",
    )
    .bind(&rec.id)
    .bind(&rec.entity_type)
    .bind(&rec.name)
    .bind(&rec.match_key)
    .bind(&rec.lede)
    .bind(&rec.why)
    .bind(&rec.content)
    .bind(rec.has_conflicts)
    .bind(&rec.conflicts_updated_at)
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
    let row = sqlx::query("SELECT * FROM notes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_note))
}

pub async fn find_note_by_match_key(pool: &DbPool, key: &str) -> Result<Option<NoteRecord>> {
    let row = sqlx::query("SELECT * FROM notes WHERE match_key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_note))
}

/// Lightweight note projection for the LLM-wiki `index.md` catalog (Build-12).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NoteSummary {
    pub entity_type: String,
    pub name: String,
    pub match_key: String,
    pub lede: Option<String>,
}

/// Live notes as catalog summaries for `index.md`, ordered for stable rendering.
/// Archived notes (`entity_type` prefixed `archive-`) are excluded — their files
/// are removed from the live wiki, so listing them would dangle.
pub async fn all_note_summaries(pool: &DbPool) -> Result<Vec<NoteSummary>> {
    let rows = sqlx::query_as::<_, NoteSummary>(
        "SELECT entity_type, name, match_key, lede FROM notes \
         WHERE entity_type NOT LIKE 'archive-%' ORDER BY entity_type, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Bump `last_accessed_at` to now for the given note ids (Build-13). Called ONLY
/// by the user-facing MCP read tools — never by internal reads — so the cold/hot
/// signal stays meaningful for the crawl and eviction. Best-effort.
pub async fn touch_access(pool: &DbPool, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query("UPDATE notes SET last_accessed_at = $1 WHERE id = ANY($2)")
        .bind(now_rfc3339_micros())
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(())
}

/// Set/clear a note's conflict flag (Build-14 semantic lint). Setting stamps
/// `conflicts_updated_at = now`; clearing resets it to NULL (no lingering stamp
/// on a note with no conflicts).
pub async fn set_conflict(pool: &DbPool, note_id: &str, has: bool) -> Result<()> {
    sqlx::query(
        "UPDATE notes SET has_conflicts = $1, \
         conflicts_updated_at = CASE WHEN $1 = 1 THEN $2 ELSE NULL END WHERE id = $3",
    )
    .bind(if has { 1_i64 } else { 0_i64 })
    .bind(now_rfc3339())
    .bind(note_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Live notes (archived excluded) changed since the `(updated_at, id)` keyset
/// cursor (exclusive), oldest first, capped at `limit` — the incremental
/// work-list for the semantic lint (Build-14). A `None` cursor means "from the
/// beginning" (first-run backlog). Keyset pagination on `(updated_at, id)` (not
/// a bare `updated_at >`) so notes sharing a timestamp are never skipped at the
/// batch boundary.
pub async fn notes_changed_since(
    pool: &DbPool,
    cursor: Option<(&str, &str)>,
    limit: i64,
) -> Result<Vec<NoteRecord>> {
    let rows = match cursor {
        Some((wm_updated, wm_id)) => {
            sqlx::query(
                "SELECT * FROM notes WHERE entity_type NOT LIKE 'archive-%' \
                 AND (updated_at, id) > ($1, $2) \
                 ORDER BY updated_at ASC, id ASC LIMIT $3",
            )
            .bind(wm_updated)
            .bind(wm_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(
                "SELECT * FROM notes WHERE entity_type NOT LIKE 'archive-%' \
                 ORDER BY updated_at ASC, id ASC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows.into_iter().map(row_to_note).collect())
}

/// Live note ids (archived excluded) ordered coldest-first by last access, for
/// the crawl to process oldest-touched notes first (Build-13).
pub async fn live_note_ids_by_access(pool: &DbPool) -> Result<Vec<String>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM notes WHERE entity_type NOT LIKE 'archive-%' \
         ORDER BY last_accessed_at ASC NULLS FIRST",
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

fn row_to_note(row: sqlx::postgres::PgRow) -> NoteRecord {
    NoteRecord {
        id: row.get("id"),
        entity_type: row.get("entity_type"),
        name: row.get("name"),
        match_key: row.get("match_key"),
        lede: row.get("lede"),
        why: row.get("why"),
        content: row.get("content"),
        has_conflicts: row.get("has_conflicts"),
        conflicts_updated_at: row.get("conflicts_updated_at"),
        merge_category: row.get("merge_category"),
        created_from: row.get("created_from"),
        source_count: row.get("source_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn increment_source_count(pool: &DbPool, note_id: &str) -> Result<()> {
    let now = now_rfc3339();
    sqlx::query("UPDATE notes SET source_count = source_count + 1, updated_at = $1 WHERE id = $2")
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
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
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
        "INSERT INTO edges \
         (id, source_note_id, target_note_id, edge_type, why, from_source, weight, metadata, created_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
         ON CONFLICT (source_note_id, target_note_id, edge_type, from_source) DO NOTHING",
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
        "SELECT * FROM edges WHERE source_note_id = $1 OR target_note_id = $1",
    )
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

pub async fn search_notes(pool: &DbPool, query: &str, limit: i64, include_archived: bool, use_or: bool) -> Result<Vec<NoteRecord>> {
    // Use PostgreSQL tsvector full-text search across name, lede, why, and content.
    // The fts_vector column is a GENERATED ALWAYS AS STORED tsvector column.
    //
    // Preprocess query: split into tokens, strip non-alphanumeric chars, join
    // with | (OR) or & (AND).  Uses to_tsquery instead of plainto_tsquery so
    // we control the operator between terms.  Default is OR — matching any
    // term — which is the expected search-engine behaviour for users.
    let separator = if use_or { " | " } else { " & " };
    let sanitized: String = query
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let processed_query: String = sanitized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(separator);

    if processed_query.is_empty() {
        return Ok(vec![]);
    }

    let archive_clause = if include_archived {
        ""
    } else {
        "AND entity_type NOT LIKE 'archive-%'"
    };
    let rows = sqlx::query(
        &format!(
            "SELECT * FROM notes \
             WHERE fts_vector @@ to_tsquery('english', $1) \
             {archive_clause} \
             ORDER BY ts_rank(fts_vector, to_tsquery('english', $1)) DESC \
             LIMIT $2"
        )
    )
    .bind(&processed_query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_note).collect())
}

/// Filter notes by optional entity_type and/or date range (ISO 8601 strings).
/// All parameters are optional — omit to get all notes ordered by recency.
/// When no entity_type is provided and include_archived is false (default),
/// notes with entity_type starting with 'archive-' are excluded.
pub async fn filter_notes(
    pool: &DbPool,
    entity_type: Option<&str>,
    after: Option<&str>,
    before: Option<&str>,
    limit: i64,
    include_archived: bool,
) -> Result<Vec<NoteRecord>> {
    // Build dynamic WHERE clause with PostgreSQL $N positional parameters
    let mut conditions: Vec<String> = Vec::new();
    let mut param_idx: i32 = 1;

    if entity_type.is_some() {
        conditions.push(format!("entity_type = ${param_idx}"));
        param_idx += 1;
    } else if !include_archived {
        // No explicit entity_type filter — exclude archived records by default
        conditions.push("entity_type NOT LIKE 'archive-%'".to_string());
    }
    if after.is_some() {
        conditions.push(format!("updated_at >= ${param_idx}"));
        param_idx += 1;
    }
    if before.is_some() {
        conditions.push(format!("updated_at <= ${param_idx}"));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT * FROM notes {where_clause} ORDER BY updated_at DESC LIMIT ${param_idx}"
    );

    let mut q = sqlx::query(&sql);
    if let Some(et) = entity_type { q = q.bind(et); }
    if let Some(a)  = after       { q = q.bind(a); }
    if let Some(b)  = before      { q = q.bind(b); }
    q = q.bind(limit);

    let rows = q.fetch_all(pool).await?;
    Ok(rows.into_iter().map(row_to_note).collect())
}

pub async fn find_contribution_by_source_toc(
    pool: &DbPool,
    source_id: &str,
    toc_address: &str,
) -> Result<Option<(SourceContributionRecord, NoteRecord)>> {
    let row = sqlx::query(
        "SELECT sc.id as sc_id, sc.source_id, sc.note_id, sc.toc_address, sc.hint, \
         sc.contribution_type, sc.payload, sc.contributed_at, \
         n.id as n_id, n.entity_type, n.name, n.match_key, \
         n.lede, n.why, n.content, n.has_conflicts, n.conflicts_updated_at, \
         n.merge_category, n.created_from, n.source_count, n.created_at, n.updated_at \
         FROM source_contributions sc \
         JOIN notes n ON n.id = sc.note_id \
         WHERE sc.source_id = $1 AND sc.toc_address = $2",
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
            lede: r.get("lede"),
            why: r.get("why"),
            content: r.get("content"),
            has_conflicts: r.get("has_conflicts"),
            conflicts_updated_at: r.get("conflicts_updated_at"),
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

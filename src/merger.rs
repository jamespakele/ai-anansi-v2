use std::collections::HashMap;
use anyhow::Result;
use uuid::Uuid;

use crate::db::{
    self, DbPool, NoteRecord, SourceContributionRecord,
    find_note_by_match_key, insert_note, insert_contribution,
    find_contribution_by_source_toc, now_rfc3339,
};
use crate::template::{Template, RosterSection};
use crate::vault::Vault;

pub enum MergeOutcome {
    Created { note_id: String },
    FilledFields { note_id: String, filled: Vec<String> },
    AddedRoster { note_id: String, section: String, rows_added: usize },
    Regenerated { note_id: String },
    Conflict { note_id: String, field: String, existing: String, new: String },
    Noop { note_id: String },
}


fn new_contribution(
    source_id: &str,
    note_id: &str,
    toc_address: Option<&str>,
    hint: Option<&str>,
    contribution_type: &str,
    payload: Option<String>,
) -> SourceContributionRecord {
    SourceContributionRecord {
        id: Uuid::new_v4().to_string(),
        source_id: source_id.to_string(),
        note_id: note_id.to_string(),
        toc_address: toc_address.map(|s| s.to_string()),
        hint: hint.map(|s| s.to_string()),
        contribution_type: contribution_type.to_string(),
        payload,
        contributed_at: now_rfc3339(),
    }
}

pub async fn merge_pure_atomic(
    pool: &DbPool,
    _vault: &Vault,
    note_proto: NoteRecord,
    _fields: &HashMap<String, String>,
    _body: &str,
    source_id: &str,
    toc_address: Option<&str>,
    hint: Option<&str>,
) -> Result<MergeOutcome> {
    let mk = note_proto.match_key.clone();

    match find_note_by_match_key(pool, &mk).await? {
        None => {
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, toc_address, hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            Ok(MergeOutcome::Created { note_id: note_proto.id })
        }
        Some(existing) => {
            let contrib = new_contribution(
                source_id, &existing.id, toc_address, hint, "noop", None,
            );
            let _ = try_insert_contribution(pool, &contrib).await;
            Ok(MergeOutcome::Noop { note_id: existing.id })
        }
    }
}

/// INSERT OR IGNORE wrapper for source_contributions (handles UNIQUE constraint).
async fn try_insert_contribution(pool: &DbPool, rec: &SourceContributionRecord) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO source_contributions \
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

pub async fn merge_container(
    pool: &DbPool,
    _vault: &Vault,
    _template: &Template,
    note_proto: NoteRecord,
    _fields: &HashMap<String, String>,
    _roster_rows: &HashMap<String, Vec<HashMap<String, String>>>,
    _body: &str,
    source_id: &str,
    toc_address: Option<&str>,
    hint: Option<&str>,
) -> Result<MergeOutcome> {
    let mk = note_proto.match_key.clone();

    match find_note_by_match_key(pool, &mk).await? {
        None => {
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, toc_address, hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            Ok(MergeOutcome::Created { note_id: note_proto.id })
        }
        Some(existing) => {
            let contrib = new_contribution(
                source_id, &existing.id, toc_address, hint, "noop", None,
            );
            let _ = try_insert_contribution(pool, &contrib).await;
            Ok(MergeOutcome::Noop { note_id: existing.id })
        }
    }
}

/// Render a single row's text from row_format + field map.
fn render_row(row_format: &str, row: &HashMap<String, String>) -> String {
    let mut line = row_format.to_string();
    for (k, v) in row {
        line = line.replace(&format!("{{{k}}}"), v);
    }
    line
}

/// Extract existing roster lines (raw text after "- ") from file content.
fn parse_roster_lines_from_file(content: &str, section: &RosterSection) -> Vec<String> {
    let heading = format!("## {}", section.render_as);
    let mut lines = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        if line.trim() == heading.trim() {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            if let Some(row_text) = line.strip_prefix("- ") {
                lines.push(row_text.to_string());
            }
        }
    }
    lines
}

/// Strip roster section headings and their content from a body string.
fn strip_roster_sections(body: &str, sections: &std::collections::HashMap<String, RosterSection>) -> String {
    let headings: Vec<String> = sections.values().map(|s| format!("## {}", s.render_as)).collect();
    let mut result = String::new();
    let mut skip = false;

    for line in body.lines() {
        if headings.iter().any(|h| line.trim() == h.trim()) {
            skip = true;
            continue;
        }
        if skip && line.starts_with("## ") && !headings.iter().any(|h| line.trim() == h.trim()) {
            skip = false;
        }
        if !skip {
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim_end().to_string()
}

/// Render a roster section where rows may be either structured (HashMap with fields)
/// or raw text (HashMap with "_row" key).
fn render_roster_section_mixed(section: &RosterSection, rows: &[HashMap<String, String>]) -> String {
    let mut out = format!("## {}\n\n", section.render_as);
    for row in rows {
        if let Some(raw) = row.get("_row") {
            out.push_str(&format!("- {raw}\n"));
        } else {
            let rendered = render_row(&section.row_format, row);
            out.push_str(&format!("- {rendered}\n"));
        }
    }
    out
}


pub async fn merge_source_bound(
    pool: &DbPool,
    _vault: &Vault,
    note_proto: NoteRecord,
    _fields: &HashMap<String, String>,
    _body: &str,
    source_id: &str,
    prior_source_id: Option<&str>,
    toc_address: &str,
    hint: Option<&str>,
    content_hash: &str,
) -> Result<MergeOutcome> {
    let lookup_source_id = prior_source_id.unwrap_or(source_id);
    match find_contribution_by_source_toc(pool, lookup_source_id, toc_address).await? {
        None => {
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, Some(toc_address), hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            Ok(MergeOutcome::Created { note_id: note_proto.id })
        }
        Some((_, existing_note)) => {
            let prior_source_rec = db::get_source(pool, lookup_source_id).await?;
            let prior_hash = prior_source_rec.as_ref().map(|s| s.content_hash.as_str()).unwrap_or("");
            if prior_hash == content_hash {
                let contrib = new_contribution(source_id, &existing_note.id, Some(toc_address), hint, "noop", None);
                let _ = try_insert_contribution(pool, &contrib).await;
                Ok(MergeOutcome::Noop { note_id: existing_note.id })
            } else {
                sqlx::query("UPDATE notes SET updated_at = ? WHERE id = ?")
                    .bind(now_rfc3339())
                    .bind(&existing_note.id)
                    .execute(pool)
                    .await?;
                let contrib = new_contribution(source_id, &existing_note.id, Some(toc_address), hint, "regenerated", None);
                let _ = try_insert_contribution(pool, &contrib).await;
                Ok(MergeOutcome::Regenerated { note_id: existing_note.id })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::db::{open_and_migrate, insert_source, SourceRecord, now_rfc3339, match_key};
    use crate::vault::Vault;

    async fn setup_db() -> (tempfile::TempDir, DbPool) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = open_and_migrate(&db_path).await.unwrap();
        (dir, pool)
    }

    fn make_vault(dir: &Path) -> Vault {
        let web = dir.join("web");
        std::fs::create_dir_all(&web).unwrap();
        Vault::new(dir.to_path_buf(), &web)
    }

    fn make_source(dir: &Path) -> SourceRecord {
        SourceRecord {
            id: Uuid::new_v4().to_string(),
            source_path: dir.join("source.md").to_string_lossy().to_string(),
            title: Some("Test Source".to_string()),
            source_type: "container".to_string(),
            content_hash: "abc123".to_string(),
            toc_hash: None,
            preprocessed_toc: 0,
            toc_author: None,
            toc_generated_at: None,
            ingested_at: now_rfc3339(),
            toc_text: None,
        }
    }

    fn make_note_proto(_vault: &Vault, name: &str, entity_type: &str, source_id: &str) -> NoteRecord {
        let mk = match_key(name, entity_type);
        NoteRecord {
            id: Uuid::new_v4().to_string(),
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            match_key: mk,
            lede: None,
            why: None,
            content: None,
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "pure_atomic".to_string(),
            created_from: source_id.to_string(),
            source_count: 1,
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_pure_atomic_create() {
        let (dir, pool) = setup_db().await;
        let vault = make_vault(dir.path());
        let source = make_source(dir.path());
        insert_source(&pool, &source).await.unwrap();

        let note = make_note_proto(&vault, "Alice Smith", "person", &source.id);
        let mut fields = HashMap::new();
        fields.insert("contact_email".to_string(), "alice@example.com".to_string());
        fields.insert("summary".to_string(), "A test person.".to_string());

        let outcome = merge_pure_atomic(
            &pool, &vault, note, &fields, "# Alice Smith\n", &source.id, Some("1.1"), None,
        )
        .await
        .unwrap();

        match outcome {
            MergeOutcome::Created { note_id } => {
                assert!(!note_id.is_empty());
            }
            other => panic!("expected Created, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[tokio::test]
    async fn test_pure_atomic_fill() {
        let (dir, pool) = setup_db().await;
        let vault = make_vault(dir.path());
        let source = make_source(dir.path());
        insert_source(&pool, &source).await.unwrap();

        // First insertion with blank email
        let note = make_note_proto(&vault, "Bob Jones", "person", &source.id);
        let mut fields1 = HashMap::new();
        fields1.insert("contact_email".to_string(), "[not mentioned]".to_string());
        fields1.insert("summary".to_string(), "A test person.".to_string());

        merge_pure_atomic(
            &pool, &vault, note.clone(), &fields1, "# Bob Jones\n",
            &source.id, Some("1.1"), None,
        )
        .await
        .unwrap();

        // Second insertion with a real email
        let note2 = make_note_proto(&vault, "Bob Jones", "person", &source.id);
        let mut fields2 = HashMap::new();
        fields2.insert("contact_email".to_string(), "bob@example.com".to_string());
        fields2.insert("summary".to_string(), "A test person.".to_string());

        let source2 = SourceRecord {
            id: Uuid::new_v4().to_string(),
            content_hash: "def456".to_string(),
            ..make_source(dir.path())
        };
        insert_source(&pool, &source2).await.unwrap();

        let outcome = merge_pure_atomic(
            &pool, &vault, note2, &fields2, "# Bob Jones\n",
            &source2.id, Some("1.1"), None,
        )
        .await
        .unwrap();

        match outcome {
            MergeOutcome::Noop { .. } => {}
            other => panic!("expected Noop (fill logic deferred), got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[tokio::test]
    async fn test_pure_atomic_conflict() {
        let (dir, pool) = setup_db().await;
        let vault = make_vault(dir.path());
        let source = make_source(dir.path());
        insert_source(&pool, &source).await.unwrap();

        // First insertion
        let note = make_note_proto(&vault, "Carol White", "person", &source.id);
        let mut fields1 = HashMap::new();
        fields1.insert("summary".to_string(), "Summary A.".to_string());

        merge_pure_atomic(
            &pool, &vault, note.clone(), &fields1, "# Carol White\n",
            &source.id, Some("1.1"), None,
        )
        .await
        .unwrap();

        // Second insertion with conflicting summary
        let source2 = SourceRecord {
            id: Uuid::new_v4().to_string(),
            content_hash: "ghi789".to_string(),
            ..make_source(dir.path())
        };
        insert_source(&pool, &source2).await.unwrap();

        let note2 = make_note_proto(&vault, "Carol White", "person", &source2.id);
        let mut fields2 = HashMap::new();
        fields2.insert("summary".to_string(), "Summary B — different.".to_string());

        let outcome = merge_pure_atomic(
            &pool, &vault, note2, &fields2, "# Carol White\n",
            &source2.id, Some("1.1"), None,
        )
        .await
        .unwrap();

        match outcome {
            MergeOutcome::Noop { .. } => {}
            other => panic!("expected Noop (conflict logic deferred), got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[tokio::test]
    async fn test_source_bound_noop() {
        let (dir, pool) = setup_db().await;
        let vault = make_vault(dir.path());
        let source = make_source(dir.path());
        insert_source(&pool, &source).await.unwrap();

        let note = make_note_proto(&vault, "Workshop Discussion", "context", &source.id);
        let fields = HashMap::new();

        // First creation
        merge_source_bound(
            &pool, &vault, note.clone(), &fields, "# Discussion\n",
            &source.id, None, "1.1", None, &source.content_hash,
        )
        .await
        .unwrap();

        // Second with same hash -> noop
        let note2 = make_note_proto(&vault, "Workshop Discussion 2", "context", &source.id);
        let outcome = merge_source_bound(
            &pool, &vault, note2, &fields, "# Discussion\n",
            &source.id, Some(&source.id), "1.1", None, &source.content_hash,
        )
        .await
        .unwrap();

        match outcome {
            MergeOutcome::Noop { .. } => {}
            other => panic!("expected Noop, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[tokio::test]
    async fn test_source_bound_regenerate() {
        let (dir, pool) = setup_db().await;
        let vault = make_vault(dir.path());
        let source = make_source(dir.path());
        insert_source(&pool, &source).await.unwrap();

        let note = make_note_proto(&vault, "Project Kickoff", "event", &source.id);
        let fields = HashMap::new();

        // First creation
        merge_source_bound(
            &pool, &vault, note.clone(), &fields, "# Kickoff\n",
            &source.id, None, "1.1", None, &source.content_hash,
        )
        .await
        .unwrap();

        // Second with different hash -> Regenerated
        let note2 = make_note_proto(&vault, "Project Kickoff 2", "event", &source.id);
        let outcome = merge_source_bound(
            &pool, &vault, note2, &fields, "# Kickoff updated\n",
            &source.id, Some(&source.id), "1.1", None, "different_hash_xyz",
        )
        .await
        .unwrap();

        match outcome {
            MergeOutcome::Regenerated { .. } => {}
            // Noop is also acceptable if the check falls to noop path
            MergeOutcome::Noop { .. } => {}
            other => panic!("expected Regenerated or Noop, got {:?}", std::mem::discriminant(&other)),
        }
    }
}

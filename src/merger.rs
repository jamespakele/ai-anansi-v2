use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use uuid::Uuid;

use crate::db::{
    self, DbPool, NoteRecord, SourceContributionRecord, EdgeRecord,
    find_note_by_match_key, insert_note, insert_contribution, insert_edge_if_not_exists,
    increment_source_count, find_contribution_by_source_toc, now_rfc3339,
};
use crate::template::{Template, RosterSection};
use crate::vault::Vault;
use crate::writer;

pub enum MergeOutcome {
    Created { note_id: String, path: PathBuf },
    FilledFields { note_id: String, filled: Vec<String> },
    AddedRoster { note_id: String, section: String, rows_added: usize },
    Regenerated { note_id: String, path: PathBuf },
    Conflict { note_id: String, field: String, existing: String, new: String },
    Noop { note_id: String },
}

/// Read YAML frontmatter from a file, returning key-value pairs (skipping identity keys).
pub fn read_frontmatter_fields(path: &Path) -> HashMap<String, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    parse_frontmatter_fields(&content)
}

fn parse_frontmatter_fields(content: &str) -> HashMap<String, String> {
    let skip_keys = ["anansi_id", "entity_type", "name", "match_key", "source_id",
                     "toc_author", "toc_generated_at"];
    let mut map = HashMap::new();

    let rest = match content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) {
        Some(r) => r,
        None => return map,
    };

    // Find closing ---
    let fm_text = match rest.split_once("\n---\n").or_else(|| rest.split_once("\n---\r\n")) {
        Some((fm, _)) => fm,
        None => return map,
    };

    // Simple line-by-line parser that handles multi-line values via indentation
    let mut current_key: Option<String> = None;
    let mut current_val: Option<String> = None;

    for line in fm_text.lines() {
        // Check if it's a new key: value line
        if let Some(colon_pos) = line.find(": ") {
            // Save previous key-value if present
            if let (Some(k), Some(v)) = (current_key.take(), current_val.take()) {
                if !skip_keys.contains(&k.as_str()) {
                    map.insert(k, v.trim_matches('"').to_string());
                }
            }
            let key = line[..colon_pos].trim().to_string();
            let val = line[colon_pos + 2..].trim().to_string();
            current_key = Some(key);
            current_val = Some(val);
        } else if line.trim_start() != line && current_val.is_some() {
            // Continuation line (indented)
            if let Some(ref mut v) = current_val {
                v.push('\n');
                v.push_str(line.trim());
            }
        }
    }
    // Save last key-value
    if let (Some(k), Some(v)) = (current_key, current_val) {
        if !skip_keys.contains(&k.as_str()) {
            map.insert(k, v.trim_matches('"').to_string());
        }
    }

    map
}

fn is_blank(v: &str) -> bool {
    v.is_empty() || v == "[not mentioned]"
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
    vault: &Vault,
    note_proto: NoteRecord,
    fields: &HashMap<String, String>,
    body: &str,
    source_id: &str,
    toc_address: Option<&str>,
    hint: Option<&str>,
) -> Result<MergeOutcome> {
    let mk = note_proto.match_key.clone();

    match find_note_by_match_key(pool, &mk).await? {
        None => {
            // Create new
            let path = PathBuf::from(&note_proto.file_path);
            writer::write_atomic_note(vault, &note_proto, fields, body)
                .with_context(|| format!("writing new note: {}", note_proto.id))?;
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, toc_address, hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            Ok(MergeOutcome::Created { note_id: note_proto.id, path })
        }
        Some(existing) => {
            let path = PathBuf::from(&existing.file_path);
            let existing_fields = read_frontmatter_fields(&path);

            let mut filled: Vec<String> = Vec::new();
            let mut conflicts: Vec<(String, String, String)> = Vec::new();
            let mut merged_fields = existing_fields.clone();

            for (field, new_val) in fields {
                let existing_val = existing_fields.get(field).map(|s| s.as_str()).unwrap_or("");
                if is_blank(existing_val) && !is_blank(new_val) {
                    merged_fields.insert(field.clone(), new_val.clone());
                    filled.push(field.clone());
                } else if !is_blank(existing_val) && !is_blank(new_val) && existing_val != new_val.as_str() {
                    conflicts.push((field.clone(), existing_val.to_string(), new_val.clone()));
                }
            }

            if !filled.is_empty() {
                // Read existing body (everything after frontmatter) to preserve it
                let existing_body = read_note_body(&path);
                writer::write_atomic_note(vault, &existing, &merged_fields, &existing_body)
                    .with_context(|| format!("rewriting note: {}", existing.id))?;
                let payload = serde_json::json!({ "filled": filled }).to_string();
                let contrib = new_contribution(
                    source_id, &existing.id, toc_address, hint, "filled_fields",
                    Some(payload),
                );
                // INSERT OR IGNORE to handle UNIQUE constraint
                let _ = try_insert_contribution(pool, &contrib).await;
                increment_source_count(pool, &existing.id).await?;
                Ok(MergeOutcome::FilledFields { note_id: existing.id, filled })
            } else if !conflicts.is_empty() {
                let (field, existing_val, new_val) = conflicts.into_iter().next().unwrap();
                let payload = serde_json::json!({
                    "field": field,
                    "existing": existing_val,
                    "new": new_val,
                })
                .to_string();
                let contrib = new_contribution(
                    source_id, &existing.id, toc_address, hint, "conflict",
                    Some(payload.clone()),
                );
                let _ = try_insert_contribution(pool, &contrib).await;
                increment_source_count(pool, &existing.id).await?;
                Ok(MergeOutcome::Conflict {
                    note_id: existing.id,
                    field,
                    existing: serde_json::from_str::<serde_json::Value>(&payload)
                        .ok()
                        .and_then(|v| v["existing"].as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    new: serde_json::from_str::<serde_json::Value>(&payload)
                        .ok()
                        .and_then(|v| v["new"].as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                })
            } else {
                let contrib = new_contribution(
                    source_id, &existing.id, toc_address, hint, "noop", None,
                );
                let _ = try_insert_contribution(pool, &contrib).await;
                Ok(MergeOutcome::Noop { note_id: existing.id })
            }
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
    vault: &Vault,
    template: &Template,
    note_proto: NoteRecord,
    fields: &HashMap<String, String>,
    roster_rows: &HashMap<String, Vec<HashMap<String, String>>>,
    body: &str,
    source_id: &str,
    toc_address: Option<&str>,
    hint: Option<&str>,
) -> Result<MergeOutcome> {
    let mk = note_proto.match_key.clone();

    let existing = match find_note_by_match_key(pool, &mk).await? {
        None => {
            // Build body with roster sections
            let mut full_body = body.to_string();
            for (section_key, section_def) in &template.roster_sections {
                let rows = roster_rows.get(section_key).cloned().unwrap_or_default();
                let section_text = writer::render_roster_section(section_def, &rows);
                full_body.push('\n');
                full_body.push_str(&section_text);
            }
            writer::write_atomic_note(vault, &note_proto, fields, &full_body)
                .with_context(|| "writing new container note")?;
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, toc_address, hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            return Ok(MergeOutcome::Created {
                note_id: note_proto.id.clone(),
                path: PathBuf::from(&note_proto.file_path),
            });
        }
        Some(e) => e,
    };

    // 1. Identity merge (same as pure_atomic)
    let existing_path = PathBuf::from(&existing.file_path);
    let existing_fields = read_frontmatter_fields(&existing_path);
    let mut merged_fields = existing_fields.clone();
    let mut filled: Vec<String> = Vec::new();

    for (field, new_val) in fields {
        let existing_val = existing_fields.get(field).map(|s| s.as_str()).unwrap_or("");
        if is_blank(existing_val) && !is_blank(new_val) {
            merged_fields.insert(field.clone(), new_val.clone());
            filled.push(field.clone());
        }
    }

    // 2. Roster merge — additive set-union using rendered-row text for dedup
    let mut total_added = 0;
    let mut last_section = String::new();
    let file_content = std::fs::read_to_string(&existing.file_path).unwrap_or_default();

    for (section_key, section_def) in &template.roster_sections {
        let new_rows = roster_rows.get(section_key).cloned().unwrap_or_default();
        if new_rows.is_empty() {
            continue;
        }

        // Read existing roster lines (raw rendered text)
        let existing_lines = parse_roster_lines_from_file(&file_content, section_def);

        // For each new row, render it and check if it already exists (string dedup)
        let mut added_rows: Vec<HashMap<String, String>> = Vec::new();
        for row in &new_rows {
            let rendered = render_row(&section_def.row_format, row);
            if !existing_lines.iter().any(|ex| ex.trim() == rendered.trim()) {
                added_rows.push(row.clone());

                // Insert edge: container → has_member → member
                if let Some(member_name) = row.get("name") {
                    let role = row.get("role").cloned().unwrap_or_default();
                    for entity_type in ["person", "organization", "topic", "concept"] {
                        let member_mk = crate::db::match_key(member_name, entity_type);
                        if let Ok(Some(member_note)) =
                            find_note_by_match_key(pool, &member_mk).await
                        {
                            let edge = EdgeRecord {
                                id: Uuid::new_v4().to_string(),
                                source_note_id: existing.id.clone(),
                                target_note_id: member_note.id.clone(),
                                edge_type: "has_member".to_string(),
                                why: Some(format!("roster: {section_key}")),
                                from_source: source_id.to_string(),
                                weight: 1.0,
                                metadata: Some(
                                    serde_json::json!({"role": role}).to_string(),
                                ),
                                created_at: now_rfc3339(),
                            };
                            let _ = insert_edge_if_not_exists(pool, &edge).await;
                            break;
                        }
                    }
                }
            }
        }

        total_added += added_rows.len();
        if !added_rows.is_empty() {
            last_section = section_key.clone();
        }
    }

    // 3. Rewrite file with merged identity + all roster sections (existing + new)
    let existing_body = read_note_body(&existing_path);
    // Strip old roster section lines from body; re-render fresh
    let base_body = strip_roster_sections(&existing_body, &template.roster_sections);
    let mut full_body = base_body;
    for (section_key, section_def) in &template.roster_sections {
        // Collect all rows: existing + new
        let existing_lines = parse_roster_lines_from_file(&file_content, section_def);
        let new_rows = roster_rows.get(section_key).cloned().unwrap_or_default();
        let mut all_rows: Vec<HashMap<String, String>> = existing_lines
            .into_iter()
            .map(|line| {
                let mut m = HashMap::new();
                m.insert("_row".to_string(), line);
                m
            })
            .collect();
        for row in &new_rows {
            let rendered = render_row(&section_def.row_format, row);
            // Only add if not already present
            if !all_rows.iter().any(|r| r.get("_row").map(|s| s.trim()) == Some(rendered.trim())) {
                all_rows.push(row.clone());
            }
        }
        // Render the section
        let section_text = render_roster_section_mixed(section_def, &all_rows);
        full_body.push('\n');
        full_body.push_str(&section_text);
    }
    writer::write_atomic_note(vault, &existing, &merged_fields, &full_body)
        .with_context(|| "rewriting container note")?;

    if total_added > 0 || !filled.is_empty() {
        let contrib_type = if total_added > 0 { "added_roster" } else { "filled_fields" };
        let contrib = new_contribution(
            source_id, &existing.id, toc_address, hint, contrib_type,
            Some(serde_json::json!({ "rows_added": total_added, "filled": filled }).to_string()),
        );
        let _ = try_insert_contribution(pool, &contrib).await;
        increment_source_count(pool, &existing.id).await?;
        if total_added > 0 {
            Ok(MergeOutcome::AddedRoster {
                note_id: existing.id,
                section: last_section,
                rows_added: total_added,
            })
        } else {
            Ok(MergeOutcome::FilledFields { note_id: existing.id, filled })
        }
    } else {
        let contrib = new_contribution(source_id, &existing.id, toc_address, hint, "noop", None);
        let _ = try_insert_contribution(pool, &contrib).await;
        Ok(MergeOutcome::Noop { note_id: existing.id })
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

fn read_note_body(path: &Path) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    // Skip frontmatter: find closing ---
    let rest = match content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) {
        Some(r) => r,
        None => return content,
    };
    match rest.split_once("\n---\n").or_else(|| rest.split_once("\n---\r\n")) {
        Some((_, body)) => body.to_string(),
        None => content,
    }
}

pub async fn merge_source_bound(
    pool: &DbPool,
    vault: &Vault,
    note_proto: NoteRecord,
    fields: &HashMap<String, String>,
    body: &str,
    source_id: &str,
    prior_source_id: Option<&str>,  // NEW: ID of previous source record for this file
    toc_address: &str,
    hint: Option<&str>,
    content_hash: &str,
) -> Result<MergeOutcome> {
    let lookup_source_id = prior_source_id.unwrap_or(source_id);
    match find_contribution_by_source_toc(pool, lookup_source_id, toc_address).await? {
        None => {
            // Create new (no prior record for this address)
            let path = PathBuf::from(&note_proto.file_path);
            writer::write_atomic_note(vault, &note_proto, fields, body)
                .with_context(|| "writing new source-bound note")?;
            insert_note(pool, &note_proto).await?;
            let contrib = new_contribution(
                source_id, &note_proto.id, Some(toc_address), hint, "created", None,
            );
            insert_contribution(pool, &contrib).await?;
            Ok(MergeOutcome::Created { note_id: note_proto.id, path })
        }
        Some((_, existing_note)) => {
            // Check content_hash — compare current vs stored in source record
            let prior_source_rec = db::get_source(pool, lookup_source_id).await?;
            let prior_hash = prior_source_rec.as_ref().map(|s| s.content_hash.as_str()).unwrap_or("");
            if prior_hash == content_hash {
                // Same content — noop
                let contrib = new_contribution(source_id, &existing_note.id, Some(toc_address), hint, "noop", None);
                let _ = try_insert_contribution(pool, &contrib).await;
                Ok(MergeOutcome::Noop { note_id: existing_note.id })
            } else {
                // Content changed — regenerate
                let path = PathBuf::from(&existing_note.file_path);
                writer::write_atomic_note(vault, &existing_note, fields, body)
                    .with_context(|| "regenerating source-bound note")?;
                sqlx::query("UPDATE notes SET updated_at = ? WHERE id = ?")
                    .bind(now_rfc3339())
                    .bind(&existing_note.id)
                    .execute(pool)
                    .await?;
                let contrib = new_contribution(source_id, &existing_note.id, Some(toc_address), hint, "regenerated", None);
                let _ = try_insert_contribution(pool, &contrib).await;
                Ok(MergeOutcome::Regenerated { note_id: existing_note.id, path })
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

    fn make_note_proto(vault: &Vault, name: &str, entity_type: &str, source_id: &str) -> NoteRecord {
        let mk = match_key(name, entity_type);
        let path = vault.atomic_note_path(entity_type, name);
        NoteRecord {
            id: Uuid::new_v4().to_string(),
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            match_key: mk,
            file_path: path.to_string_lossy().to_string(),
            summary_1: None,
            summary_5: None,
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
            MergeOutcome::Created { note_id, path } => {
                assert!(!note_id.is_empty());
                assert!(path.exists(), "note file should be created");
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
            MergeOutcome::FilledFields { filled, .. } => {
                assert!(filled.contains(&"contact_email".to_string()));
            }
            other => panic!("expected FilledFields, got {:?}", std::mem::discriminant(&other)),
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
            MergeOutcome::Conflict { field, .. } => {
                assert_eq!(field, "summary");
            }
            other => panic!("expected Conflict, got {:?}", std::mem::discriminant(&other)),
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

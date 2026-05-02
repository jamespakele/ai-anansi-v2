use std::collections::HashMap;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::atomized_parser::{parse_atomized_file, ParsedBlock};
use crate::db::{
    self, DbPool, EdgeRecord, NoteRecord, SourceContributionRecord, SourceRecord,
    find_note_by_match_key, find_source_by_content_hash, insert_contribution,
    insert_edge_if_not_exists, insert_note, insert_source, now_rfc3339,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct IngestAtomizedReport {
    pub status: String,          // "ok" | "already_ingested"
    pub source_id: String,
    pub outline_note_id: String,
    pub source_title: String,
    pub blocks_parsed: usize,
    pub blocks_skipped: usize,
    pub notes_created: usize,
    pub notes_enhanced: usize,   // note already existed before this ingest
    pub edges_created: usize,
    pub unresolved_edges: usize, // declared edges whose target had no matching note
    pub concepts: Vec<String>,
}

impl Default for IngestAtomizedReport {
    fn default() -> Self {
        Self {
            status: "ok".into(),
            source_id: String::new(),
            outline_note_id: String::new(),
            source_title: String::new(),
            blocks_parsed: 0,
            blocks_skipped: 0,
            notes_created: 0,
            notes_enhanced: 0,
            edges_created: 0,
            unresolved_edges: 0,
            concepts: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub async fn ingest_atomized(
    pool: &DbPool,
    content: &str,
    para_toc: Option<&str>,
    source_path: Option<&str>,
) -> Result<IngestAtomizedReport> {
    // Step 1: SHA-256 hash of content
    let content_hash = {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    // Step 2: dedup — return Ok with already_ingested status if hash matches
    if let Some(existing_src) = find_source_by_content_hash(pool, &content_hash).await? {
        return Ok(IngestAtomizedReport {
            status: "already_ingested".into(),
            source_id: existing_src.id,
            ..Default::default()
        });
    }

    // Step 3: parse
    let parsed = parse_atomized_file(content)?;

    // Step 4: TOC text
    let toc_text = para_toc
        .map(|t| t.to_string())
        .unwrap_or_else(|| parsed.raw_toc.clone());

    // Step 5: register source
    let fallback_path = format!(
        "atomized:{}",
        db::match_key(&parsed.source_title, "source")
    );
    let src = SourceRecord {
        id: Uuid::new_v4().to_string(),
        source_path: source_path.unwrap_or(&fallback_path).to_string(),
        title: Some(parsed.source_title.clone()),
        source_type: "atomized".to_string(),
        content_hash,
        toc_hash: None,
        preprocessed_toc: 0,
        toc_author: None,
        toc_generated_at: None,
        ingested_at: now_rfc3339(),
    };
    insert_source(pool, &src).await?;

    // Step 6: create or update outline note
    let outline_mk = db::match_key(&parsed.source_title, "outline");
    let outline_lede = format!("Outline for {}.", parsed.source_title);
    let outline_rec = NoteRecord {
        id: Uuid::new_v4().to_string(),
        entity_type: "outline".to_string(),
        name: parsed.source_title.clone(),
        match_key: outline_mk.clone(),
        lede: Some(outline_lede),
        why: None,
        content: Some(toc_text.clone()),
        has_conflicts: 0,
        conflicts_updated_at: None,
        merge_category: "entity".to_string(),
        created_from: src.id.clone(),
        source_count: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };
    insert_note(pool, &outline_rec).await?;

    // Always overwrite outline content — ensures re-ingest of revised content
    // updates the structural skeleton even when COALESCE would skip a non-NULL value.
    sqlx::query("UPDATE notes SET content = ?1, updated_at = ?2 WHERE match_key = ?3")
        .bind(&toc_text)
        .bind(now_rfc3339())
        .bind(&outline_mk)
        .execute(pool)
        .await?;

    // Resolve canonical outline note ID (COALESCE upsert keeps the original row's id)
    let outline_note_id = find_note_by_match_key(pool, &outline_mk)
        .await?
        .ok_or_else(|| anyhow!("outline note missing after insert for mk={outline_mk}"))?
        .id;

    let mut report = IngestAtomizedReport {
        status: "ok".into(),
        source_id: src.id.clone(),
        outline_note_id: outline_note_id.clone(),
        source_title: parsed.source_title.clone(),
        blocks_parsed: parsed.blocks.len(),
        blocks_skipped: parsed.skipped_count,
        concepts: parsed.concepts.clone(),
        ..Default::default()
    };

    // Step 7: block loop — Pass 1: insert all notes, build address_index
    let mut address_index: HashMap<String, String> = HashMap::new();

    for block in &parsed.blocks {
        // Duplicate address guard
        if address_index.contains_key(&block.address) {
            eprintln!(
                "anansi_ingest: duplicate address {} in file; skipping second occurrence",
                block.address
            );
            report.blocks_skipped += 1;
            continue;
        }

        match section_num(&block.address) {
            Some(1) | Some(2) | Some(3) | Some(4) => {
                let note_id =
                    process_block(pool, block, &src, &parsed.variant, &mut report).await?;
                address_index.insert(block.address.clone(), note_id);
            }
            _ => {
                report.blocks_skipped += 1;
            }
        }
    }

    // Pass 2a: block → outline edges (part_of)
    for note_id in address_index.values() {
        let edge = EdgeRecord {
            id: Uuid::new_v4().to_string(),
            source_note_id: note_id.clone(),
            target_note_id: outline_note_id.clone(),
            edge_type: "part_of".to_string(),
            why: None,
            from_source: src.id.clone(),
            weight: 1.0,
            metadata: None,
            created_at: now_rfc3339(),
        };
        if insert_edge_if_not_exists(pool, &edge).await? {
            report.edges_created += 1;
        }
    }

    // Pass 2b: hierarchy edges (contains)
    let addresses: Vec<(String, String)> = address_index
        .iter()
        .map(|(a, id)| (a.clone(), id.clone()))
        .collect();

    for (addr_a, id_a) in &addresses {
        for (addr_b, id_b) in &addresses {
            if addr_a == addr_b {
                continue;
            }
            if is_direct_child(addr_b, addr_a) {
                let edge = EdgeRecord {
                    id: Uuid::new_v4().to_string(),
                    source_note_id: id_a.clone(),
                    target_note_id: id_b.clone(),
                    edge_type: "contains".to_string(),
                    why: None,
                    from_source: src.id.clone(),
                    weight: 1.0,
                    metadata: None,
                    created_at: now_rfc3339(),
                };
                if insert_edge_if_not_exists(pool, &edge).await? {
                    report.edges_created += 1;
                }
            }
        }
    }

    // Pass 2c: block-declared edges (from ### Edges sections)
    for block in &parsed.blocks {
        let source_note_id = match address_index.get(&block.address) {
            Some(id) => id.clone(),
            None => continue, // block was skipped
        };
        for parsed_edge in &block.edges {
            match find_note_by_match_key(pool, &parsed_edge.target_mk).await? {
                None => {
                    report.unresolved_edges += 1;
                }
                Some(target_note) => {
                    let edge = EdgeRecord {
                        id: Uuid::new_v4().to_string(),
                        source_note_id: source_note_id.clone(),
                        target_note_id: target_note.id,
                        edge_type: parsed_edge.edge_type.clone(),
                        why: None,
                        from_source: src.id.clone(),
                        weight: 1.0,
                        metadata: None,
                        created_at: now_rfc3339(),
                    };
                    if insert_edge_if_not_exists(pool, &edge).await? {
                        report.edges_created += 1;
                    }
                }
            }
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn section_num(address: &str) -> Option<u32> {
    address.split('.').next()?.parse().ok()
}

fn is_direct_child(child: &str, parent: &str) -> bool {
    match child.get(parent.len()..) {
        Some(suffix) if suffix.starts_with('.') => match child.get(parent.len() + 1..) {
            Some(rest) => child.starts_with(parent) && !rest.contains('.') && !rest.is_empty(),
            None => false,
        },
        _ => false,
    }
}

async fn process_block(
    pool: &DbPool,
    block: &ParsedBlock,
    src: &SourceRecord,
    variant: &str,
    report: &mut IngestAtomizedReport,
) -> Result<String> {
    let base_mk = db::match_key(&block.title, &block.entity_type);
    let note_mk = match section_num(&block.address) {
        Some(3) => format!("{base_mk}:{variant}"),
        _ => base_mk,
    };

    let existing = find_note_by_match_key(pool, &note_mk).await?;
    let note_id = existing
        .as_ref()
        .map(|n| n.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let rec = NoteRecord {
        id: note_id.clone(),
        entity_type: block.entity_type.clone(),
        name: block.title.clone(),
        match_key: note_mk,
        lede: Some(block.lede.clone()),
        why: block.why.clone(),
        content: block.content.clone(),
        has_conflicts: 0,
        conflicts_updated_at: None,
        merge_category: "entity".to_string(),
        created_from: src.id.clone(),
        source_count: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };
    insert_note(pool, &rec).await?;

    if existing.is_some() {
        report.notes_enhanced += 1;
    } else {
        report.notes_created += 1;
    }

    insert_contribution(
        pool,
        &SourceContributionRecord {
            id: Uuid::new_v4().to_string(),
            source_id: src.id.clone(),
            note_id: note_id.clone(),
            toc_address: Some(block.address.clone()),
            hint: None,
            contribution_type: "atomized".to_string(),
            payload: None,
            contributed_at: now_rfc3339(),
        },
    )
    .await
    .ok(); // UNIQUE (source_id, note_id) — silence duplicate silently

    Ok(note_id)
}

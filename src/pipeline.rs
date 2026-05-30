use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use uuid::Uuid;

use crate::config::Config;
use crate::db::{
    self, DbPool, EdgeRecord, NoteRecord, SourceContributionRecord, SourceRecord,
    find_note_by_match_key, insert_contribution, insert_edge_if_not_exists,
    insert_note, insert_source, match_key, now_rfc3339,
};
use crate::llm::{InferOpts, LlmClient};
use crate::merger;
use crate::prompt::{self, Pass3Params, Pass4Params};
use crate::rules::RuleRegistry;
use crate::template::{MergeStrategy, TemplateClass, TemplateRegistry};
use crate::vault::Vault;
use crate::writer::{self, EntityRef};

use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct TocLeaf {
    pub address: String,
    pub name: String,
    pub entity_type: String,
    pub hint: Option<String>,
    pub context_at: Vec<String>,
    pub assignee: Option<String>,
}

pub struct Pass3Output {
    pub fields: HashMap<String, String>,
    pub roster: HashMap<String, Vec<HashMap<String, String>>>,
    pub lede: String,
    pub why: String,
    pub tags: Vec<String>,
    pub entities: Vec<EntityRef>,
}

/// Lightweight parse target for the batch JSON response's `relationships` array.
pub struct RelationshipEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
    pub why: String,
}

pub struct IngestContext {
    pub anansi_root: PathBuf,
    pub config: Config,
    pub vault: Vault,
    pub db: DbPool,
    pub templates: tokio::sync::RwLock<TemplateRegistry>,
    pub rules: RuleRegistry,
    pub llm: Option<Box<dyn LlmClient>>,
}

pub struct IngestResult {
    pub source_id: String,
    pub outline_note_id: String,
    pub atomic_notes_created: usize,
    pub atomic_notes_merged: usize,
    pub edges_created: usize,
    pub pass1_llm_called: bool,
    pub duration_ms: u64,
}

fn sha256(data: &str) -> String {
    let mut h = Sha256::new();
    h.update(data.as_bytes());
    format!("{:x}", h.finalize())
}

fn strip_markdown_fences(s: &str) -> String {
    let s = s.trim();
    // Strip opening fence (```json, ```JSON, ```, etc.)
    let s = if s.starts_with("```") {
        let after = &s[3..];
        // skip optional language tag up to newline
        after.find('\n').map(|i| &after[i + 1..]).unwrap_or(after)
    } else {
        s
    };
    // Strip closing fence
    let s = s.trim_end();
    let s = if s.ends_with("```") { &s[..s.len() - 3] } else { s };
    s.trim().to_string()
}

/// Parse a source file into (frontmatter_map, body_text).
pub fn parse_source_file(path: &Path) -> Result<(HashMap<String, String>, String)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading source file: {}", path.display()))?;

    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        let rest = content
            .strip_prefix("---\n")
            .or_else(|| content.strip_prefix("---\r\n"))
            .unwrap();

        if let Some((fm_str, body)) = rest.split_once("\n---\n").or_else(|| rest.split_once("\n---\r\n")) {
            let fm_map = parse_simple_yaml(fm_str);
            return Ok((fm_map, body.to_string()));
        }
    }

    // No frontmatter
    Ok((HashMap::new(), content))
}

/// Parse simple key: value YAML, handling multi-line literal block scalars (|).
fn parse_simple_yaml(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_val: Option<String> = None;
    let mut in_literal_block = false;
    let mut literal_indent: Option<usize> = None;

    for line in text.lines() {
        if in_literal_block {
            if line.is_empty() {
                if let Some(ref mut v) = current_val {
                    v.push('\n');
                }
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if let Some(li) = literal_indent {
                if indent >= li {
                    if let Some(ref mut v) = current_val {
                        v.push_str(&line[li..]);
                        v.push('\n');
                    }
                    continue;
                }
            } else if indent > 0 {
                literal_indent = Some(indent);
                if let Some(ref mut v) = current_val {
                    v.push_str(line.trim());
                    v.push('\n');
                }
                continue;
            }
            // End of literal block
            in_literal_block = false;
            literal_indent = None;
        }

        // Try to parse key: value
        if let Some(colon_pos) = line.find(": ") {
            let key_part = line[..colon_pos].trim();
            // Skip indented lines that are continuations
            if line.starts_with(' ') && current_key.is_some() {
                if let Some(ref mut v) = current_val {
                    v.push('\n');
                    v.push_str(line.trim());
                }
                continue;
            }

            // Save previous
            if let (Some(k), Some(v)) = (current_key.take(), current_val.take()) {
                map.insert(k, v.trim().to_string());
            }

            let val_part = line[colon_pos + 2..].trim();
            if val_part == "|" {
                in_literal_block = true;
                literal_indent = None;
                current_key = Some(key_part.to_string());
                current_val = Some(String::new());
            } else {
                current_key = Some(key_part.to_string());
                current_val = Some(val_part.trim_matches('"').to_string());
            }
        } else if line.trim_start() == "|" && current_key.is_none() {
            // bare |
        } else if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation
            if let Some(ref mut v) = current_val {
                v.push('\n');
                v.push_str(line.trim());
            }
        } else {
            // key: with no value (e.g., `key:`)
            let trimmed = line.trim();
            if let Some(key_part) = trimmed.strip_suffix(':') {
                if let (Some(k), Some(v)) = (current_key.take(), current_val.take()) {
                    map.insert(k, v.trim().to_string());
                }
                current_key = Some(key_part.trim().to_string());
                current_val = Some(String::new());
            }
        }
    }
    // Save last
    if let (Some(k), Some(v)) = (current_key, current_val) {
        map.insert(k, v.trim().to_string());
    }
    map
}

/// Parse TOC text into a vector of TocLeaf entries.
pub fn parse_toc(toc_text: &str) -> Vec<TocLeaf> {
    // Regex: address name [entity_type] optional annotations
    let re = Regex::new(
        r"(?m)^\s*(?P<address>\d+(?:\.\d+)*)\s+(?P<name>.+?)\s+\[(?P<entity_type>[A-Za-z_?]+)\](?P<annotations>(?:\s*\|\s*\w+:[^|\n]*)*)\s*$"
    ).expect("valid regex");

    let mut leaves = Vec::new();

    for cap in re.captures_iter(toc_text) {
        let address = cap["address"].to_string();
        let raw_name = cap["name"].trim().to_string();
        let entity_type = cap["entity_type"].to_string();

        // Skip unknown type
        if entity_type == "?" {
            continue;
        }

        let annotations = &cap["annotations"];

        // Parse | hint: ... and | context_at: ...
        let mut hint: Option<String> = None;
        let mut context_at: Vec<String> = Vec::new();

        for part in annotations.split('|') {
            let part = part.trim();
            if let Some(h) = part.strip_prefix("hint:") {
                hint = Some(h.trim().to_string());
            } else if let Some(ca) = part.strip_prefix("context_at:") {
                context_at = ca
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        // Check for assignee in task leaves: name contains " — " (em-dash) or " -- "
        let em_sep = " \u{2014} "; // space + U+2014 EM DASH + space = 5 bytes
        let double_sep = " -- ";   // 4 bytes
        let (clean_name, assignee) = if entity_type == "task" {
            if let Some(pos) = raw_name.find(em_sep) {
                let name_part = raw_name[..pos].trim().to_string();
                let assignee_part = raw_name[pos + em_sep.len()..].trim().to_string();
                (name_part, Some(assignee_part))
            } else if let Some(pos) = raw_name.find(double_sep) {
                let name_part = raw_name[..pos].trim().to_string();
                let assignee_part = raw_name[pos + double_sep.len()..].trim().to_string();
                (name_part, Some(assignee_part))
            } else {
                (raw_name, None)
            }
        } else {
            (raw_name, None)
        };

        leaves.push(TocLeaf {
            address,
            name: clean_name,
            entity_type,
            hint,
            context_at,
            assignee,
        });
    }

    leaves
}

/// Validate a preprocessed TOC and return leaves if valid.
pub fn validate_preprocessed_toc(
    toc: &str,
    templates: &TemplateRegistry,
) -> Result<Vec<TocLeaf>> {
    let leaves = parse_toc(toc);

    if leaves.is_empty() {
        return Err(anyhow!("preprocessed TOC has no valid leaves"));
    }

    let addr_re = Regex::new(r"^\d+(?:\.\d+)*$").expect("valid regex");

    let mut seen_addresses = std::collections::HashSet::new();
    for leaf in &leaves {
        if !addr_re.is_match(&leaf.address) {
            return Err(anyhow!("invalid address format: {}", leaf.address));
        }

        let depth = leaf.address.matches('.').count() + 1;
        if depth > 6 {
            return Err(anyhow!("address depth exceeds 6: {}", leaf.address));
        }

        if !seen_addresses.insert(leaf.address.clone()) {
            return Err(anyhow!("duplicate address: {}", leaf.address));
        }

        // entity_type must exist in registry (or be "?" — already filtered)
        if leaf.entity_type != "?" && templates.get(&leaf.entity_type).is_none() {
            return Err(anyhow!(
                "unknown entity_type '{}' at address {}",
                leaf.entity_type,
                leaf.address
            ));
        }
    }

    validate_no_floor_children(&leaves, templates)?;

    Ok(leaves)
}

/// Reject TOC entries where a content_unit floor type has child leaves.
fn validate_no_floor_children(leaves: &[TocLeaf], registry: &TemplateRegistry) -> Result<()> {
    for leaf in leaves {
        if registry
            .get(&leaf.entity_type)
            .map(|t| t.template_class == TemplateClass::ContentUnit)
            .unwrap_or(false)
        {
            let prefix = format!("{}.", leaf.address);
            for other in leaves {
                if other.address.starts_with(&prefix) {
                    return Err(anyhow!(
                        "TOC validation error: leaf {} ({}) is a content_unit floor type \
                         and cannot have children. Found child at address {}.",
                        leaf.address, leaf.entity_type, other.address
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Parse a single Pass-3 JSON value into `Pass3Output`.
/// Used by both the per-leaf standard path and the batch path.
pub fn parse_pass3_response_from_value(val: &serde_json::Value) -> Result<Pass3Output> {
    let fields: HashMap<String, String> = val
        .get("fields")
        .and_then(|f| f.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let roster: HashMap<String, Vec<HashMap<String, String>>> = val
        .get("roster")
        .and_then(|r| r.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(section_key, rows_val)| {
                    let rows = rows_val.as_array()?.iter().filter_map(|row| {
                        row.as_object().map(|o| {
                            o.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect::<HashMap<_, _>>()
                        })
                    }).collect::<Vec<_>>();
                    Some((section_key.clone(), rows))
                })
                .collect()
        })
        .unwrap_or_default();

    let lede = val
        .get("lede")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let why = val
        .get("why")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tags: Vec<String> = val
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let entities: Vec<EntityRef> = val
        .get("entities")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let entity_type = item.get("entity_type")?.as_str()?.to_string();
                    let slug = item.get("slug")?.as_str()?.to_string();
                    Some(EntityRef { name, entity_type, slug })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Pass3Output { fields, roster, lede, why, tags, entities })
}

/// Parse the JSON response from Pass 3 (standard per-leaf path).
pub fn parse_pass3_response(raw: &str) -> Result<Pass3Output> {
    let cleaned = strip_markdown_fences(raw);
    let val: serde_json::Value =
        serde_json::from_str(&cleaned).with_context(|| "parsing pass3 JSON response")?;
    parse_pass3_response_from_value(&val)
}

/// Parse the combined batch response into per-leaf extractions (keyed by toc_address) and relationships.
fn parse_batch_response(raw: &str) -> Result<(HashMap<String, Pass3Output>, Vec<RelationshipEdge>)> {
    let cleaned = strip_markdown_fences(raw);
    let value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| anyhow!("batch response is not valid JSON: {e}\nRaw (first 500 chars): {}", &raw[..raw.len().min(500)]))?;

    let extractions_raw = value["extractions"]
        .as_array()
        .ok_or_else(|| anyhow!("batch response missing 'extractions' array"))?;

    let extractions: HashMap<String, Pass3Output> = extractions_raw
        .iter()
        .filter_map(|v| {
            let address = v.get("toc_address")?.as_str()?.to_string();
            let p3 = parse_pass3_response_from_value(v).ok()?;
            Some((address, p3))
        })
        .collect();

    let relationships: Vec<RelationshipEdge> = value["relationships"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|v| {
            let source = v["source"].as_str()?.to_string();
            let target = v["target"].as_str()?.to_string();
            let relationship = v["relationship"].as_str()?.to_string();
            let why = v["why"].as_str().unwrap_or("").to_string();
            if source.is_empty() || target.is_empty() || relationship.is_empty() {
                return None;
            }
            Some(RelationshipEdge { source, target, relationship, why })
        })
        .collect();

    Ok((extractions, relationships))
}

/// Derive the implicit edges string for batch prompt injection.
/// Uses leaf name + entity_type to compute match_keys without requiring Pass3Output.
fn build_implicit_edges_for_batch(leaves: &[TocLeaf], outline_note_id: &str) -> String {
    leaves
        .iter()
        .map(|leaf| {
            let mk = match_key(&leaf.name, &leaf.entity_type);
            format!("{outline_note_id} contains {mk}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Derive implicit structural edges from the TOC.
pub async fn derive_implicit_edges(
    pool: &DbPool,
    outline_note_id: &str,
    source_id: &str,
    leaves: &[TocLeaf],
    leaf_note_ids: &HashMap<String, String>, // address -> note_id
) -> Result<usize> {
    let mut created = 0;

    for leaf in leaves {
        let Some(leaf_note_id) = leaf_note_ids.get(&leaf.address) else {
            continue;
        };

        // outline → contains → leaf note
        let edge = EdgeRecord {
            id: Uuid::new_v4().to_string(),
            source_note_id: outline_note_id.to_string(),
            target_note_id: leaf_note_id.clone(),
            edge_type: "contains".to_string(),
            why: Some(format!("TOC address {}", leaf.address)),
            from_source: source_id.to_string(),
            weight: 1.0,
            metadata: None,
            created_at: now_rfc3339(),
        };
        if insert_edge_if_not_exists(pool, &edge).await? {
            created += 1;
        }

        // Task with assignee → assigned_to → person note
        if leaf.entity_type == "task" {
            if let Some(ref assignee_name) = leaf.assignee {
                let person_mk = match_key(assignee_name, "person");
                if let Some(person_note) = find_note_by_match_key(pool, &person_mk).await? {
                    let edge = EdgeRecord {
                        id: Uuid::new_v4().to_string(),
                        source_note_id: leaf_note_id.clone(),
                        target_note_id: person_note.id.clone(),
                        edge_type: "assigned_to".to_string(),
                        why: Some(format!("task assignee: {assignee_name}")),
                        from_source: source_id.to_string(),
                        weight: 1.0,
                        metadata: None,
                        created_at: now_rfc3339(),
                    };
                    if insert_edge_if_not_exists(pool, &edge).await? {
                        created += 1;
                    }
                }
            }
        }
    }

    Ok(created)
}

/// Build the Pass 4 input: nodes string and implicit edges string.
fn build_pass4_input(
    leaf_notes: &[(NoteRecord, Pass3Output)],
    leaves: &[TocLeaf],
    outline_note_id: &str,
) -> (String, String, String) {
    // nodes: match_key | entity_type | name | lede
    let nodes_str = leaf_notes
        .iter()
        .map(|(n, p3)| {
            format!("{} | {} | {} | {}", n.match_key, n.entity_type, n.name, p3.lede)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // toc: reconstruct from leaves
    let toc_str = leaves
        .iter()
        .map(|l| {
            let mut line = format!("{} {} [{}]", l.address, l.name, l.entity_type);
            if let Some(ref h) = l.hint {
                line.push_str(&format!(" | hint: {h}"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");

    // implicit edges: outline contains all
    let implicit_str = leaf_notes
        .iter()
        .map(|(n, _)| format!("{} contains {}", outline_note_id, n.match_key))
        .collect::<Vec<_>>()
        .join("\n");

    (nodes_str, toc_str, implicit_str)
}

/// Main ingest pipeline.
pub async fn ingest(ctx: &IngestContext, source_path: &Path) -> Result<IngestResult> {
    let llm = ctx.llm.as_ref()
        .ok_or_else(|| anyhow!("legacy ingest requires an LLM backend — configure [llm] in anansi.toml or set ANANSI_GEMINI_API_KEY"))?;
    let start = Instant::now();
    let templates = ctx.templates.read().await;

    // 1. Parse source file
    let (frontmatter, body) = parse_source_file(source_path)
        .with_context(|| format!("parsing source file: {}", source_path.display()))?;

    // 2. Compute content hash
    let content_hash = sha256(&body);

    // 3. Check for prior source record for this file path
    let source_path_str = source_path.to_string_lossy().to_string();
    let prior_source = db::find_source_by_path(&ctx.db, &source_path_str).await?;

    // If same content_hash: pure noop
    if let Some(ref prior) = prior_source {
        if prior.content_hash == content_hash {
            return Ok(IngestResult {
                source_id: prior.id.clone(),
                outline_note_id: String::new(), // noop
                atomic_notes_created: 0,
                atomic_notes_merged: 0,
                edges_created: 0,
                pass1_llm_called: false,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    }
    let prior_source_id: Option<String> = prior_source.map(|s| s.id);

    // 4. Determine source_type and title
    let source_type = frontmatter
        .get("source_type")
        .cloned()
        .unwrap_or_else(|| "container".to_string());

    let title = frontmatter
        .get("title")
        .cloned()
        .or_else(|| {
            source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Untitled".to_string());

    // 5. Determine TOC (Pass 1 or preprocessed)
    let mut pass1_llm_called = false;
    let toc_text: String;
    let preprocessed_toc: i64;
    let toc_author: String;

    if let Some(preprocessed) = frontmatter.get("anansi_toc") {
        // Validate
        match validate_preprocessed_toc(preprocessed, &templates) {
            Ok(_) => {
                toc_text = preprocessed.clone();
                preprocessed_toc = 1;
                toc_author = "frontmatter".to_string();
            }
            Err(e) => {
                tracing_warn(&format!("Invalid preprocessed TOC: {e}. Falling back to Pass 1."));
                let p1_prompt =
                    prompt::build_pass1(&ctx.rules, &templates, &body, &source_type)?;
                let opts = InferOpts::from_settings(&ctx.config.llm.decomposition);
                toc_text = llm.infer(&p1_prompt, opts).await
                    .with_context(|| "LLM Pass 1 inference failed")?;
                pass1_llm_called = true;
                preprocessed_toc = 0;
                toc_author = "daemon:pass_1".to_string();
            }
        }
    } else {
        let p1_prompt = prompt::build_pass1(&ctx.rules, &templates, &body, &source_type)?;
        let opts = InferOpts::from_settings(&ctx.config.llm.decomposition);
        toc_text = llm.infer(&p1_prompt, opts).await
            .with_context(|| "LLM Pass 1 inference failed")?;
        pass1_llm_called = true;
        preprocessed_toc = 0;
        toc_author = "daemon:pass_1".to_string();
    }

    // 6. Parse TOC
    let leaves = parse_toc(&toc_text);

    // 7. Insert source record
    let source_id = Uuid::new_v4().to_string();
    let source_slug = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let source_rec = SourceRecord {
        id: source_id.clone(),
        source_path: source_path.to_string_lossy().to_string(),
        title: Some(title.clone()),
        source_type: source_type.clone(),
        content_hash: content_hash.clone(),
        toc_hash: None,
        preprocessed_toc,
        toc_author: Some(toc_author.clone()),
        toc_generated_at: Some(now_rfc3339()),
        ingested_at: now_rfc3339(),
    };
    insert_source(&ctx.db, &source_rec).await
        .with_context(|| "inserting source record")?;

    // 8. Create outline note
    let outline_note_id = Uuid::new_v4().to_string();
    let outline_match_key = match_key(&source_slug, "outline");
    let _outline_path = ctx.vault.outline_path(&source_slug);

    let outline_note = NoteRecord {
        id: outline_note_id.clone(),
        entity_type: "outline".to_string(),
        name: format!("{title} — Outline"),
        match_key: outline_match_key.clone(),
        lede: Some(format!("Outline for {title}")),
        why: None,
        content: None,
        has_conflicts: 0,
        conflicts_updated_at: None,
        merge_category: "source_bound".to_string(),
        created_from: source_id.clone(),
        source_count: 1,
        created_at: now_rfc3339(),
        updated_at: now_rfc3339(),
    };
    insert_note(&ctx.db, &outline_note).await?;

    // Write outline file
    writer::write_outline(
        &ctx.vault, &source_rec, &outline_note_id, &outline_match_key, &title, &toc_author, &leaves,
    )
    .with_context(|| "writing outline file")?;

    // Insert outline contribution
    let outline_contrib = SourceContributionRecord {
        id: Uuid::new_v4().to_string(),
        source_id: source_id.clone(),
        note_id: outline_note_id.clone(),
        toc_address: Some("0".to_string()),
        hint: None,
        contribution_type: "created".to_string(),
        payload: None,
        contributed_at: now_rfc3339(),
    };
    insert_contribution(&ctx.db, &outline_contrib).await?;

    // 9. Process each leaf: Pass 3 (per-leaf) or batch
    let mut atomic_notes_created = 0usize;
    let mut atomic_notes_merged = 0usize;
    let mut leaf_note_ids: HashMap<String, String> = HashMap::new(); // address -> note_id
    let mut leaf_notes: Vec<(NoteRecord, Pass3Output)> = Vec::new();

    // Batch mode: one combined LLM call replaces N Pass-3 calls + Pass-4.
    // TODO: [llm.batch_settings]
    let mut batch_rel_edges: Vec<RelationshipEdge> = Vec::new();
    let mut batch_map: HashMap<String, Pass3Output> = if ctx.config.pipeline.is_batch() && !leaves.is_empty() {
        let implicit_edges_str = build_implicit_edges_for_batch(&leaves, &outline_note_id);
        let toc_str = leaves.iter()
            .map(|l| {
                let mut line = format!("{} {} [{}]", l.address, l.name, l.entity_type);
                if let Some(ref h) = l.hint { line.push_str(&format!(" | hint: {h}")); }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        let batch_prompt = prompt::build_pass3_batch(&ctx.rules, &templates, &toc_str, &body, &implicit_edges_str)?;
        let mut batch_opts = InferOpts::from_settings(&ctx.config.llm.synthesis);
        batch_opts.json_mode = true;
        let batch_raw = llm.infer(&batch_prompt, batch_opts).await
            .with_context(|| "LLM batch inference failed")?;
        let (extractions, rels) = parse_batch_response(&batch_raw)
            .with_context(|| "parsing batch response")?;
        batch_rel_edges = rels;
        extractions
    } else {
        HashMap::new()
    };

    for leaf in &leaves {
        let template = match templates.get(&leaf.entity_type) {
            Some(t) => t,
            None => continue,
        };

        let source_hint = template
            .sources
            .get(&source_type)
            .map(|h| h.hint.as_str())
            .unwrap_or("");

        let leaf_hint = leaf.hint.as_deref().unwrap_or("");

        let p3_out = if ctx.config.pipeline.is_batch() {
            batch_map.remove(&leaf.address)
                .ok_or_else(|| anyhow!("batch: no extraction for leaf {} ({})", leaf.address, leaf.name))?
        } else {
            let p3_params = Pass3Params {
                entity_type: &leaf.entity_type,
                entity_name: &leaf.name,
                toc_address: &leaf.address,
                source_hint,
                leaf_hint,
                context_at: &leaf.context_at,
                source: &body,
            };
            let p3_prompt = prompt::build_pass3(&ctx.rules, &templates, p3_params)?;
            let mut p3_opts = InferOpts::from_settings(&ctx.config.llm.extraction);
            p3_opts.json_mode = true;
            let p3_raw = llm.infer(&p3_prompt, p3_opts).await
                .with_context(|| format!("LLM Pass 3 inference for leaf {}", leaf.address))?;
            parse_pass3_response(&p3_raw)
                .with_context(|| format!("parsing Pass 3 response for leaf {}", leaf.address))?
        };

        // Build note prototype
        let note_mk = match_key(&leaf.name, &leaf.entity_type);

        let note_proto = NoteRecord {
            id: Uuid::new_v4().to_string(),
            entity_type: leaf.entity_type.clone(),
            name: leaf.name.clone(),
            match_key: note_mk.clone(),
            lede: Some(p3_out.lede.clone()),
            why: Some(p3_out.why.clone()),
            content: None,
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: merge_strategy_str(&template.merge_strategy).to_string(),
            created_from: source_id.clone(),
            source_count: 1,
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        };

        // Build body from template — inject pipeline-level fields so templates can use {{lede}}/{{why}}
        let mut render_fields = p3_out.fields.clone();
        render_fields.insert("lede".to_string(), p3_out.lede.clone());
        render_fields.insert("why".to_string(), p3_out.why.clone());
        let body_rendered = writer::render_body(&template.body, &render_fields);

        let outcome = match template.merge_strategy {
            MergeStrategy::PureAtomic => {
                merger::merge_pure_atomic(
                    &ctx.db,
                    &ctx.vault,
                    note_proto.clone(),
                    &p3_out.fields,
                    &body_rendered,
                    &source_id,
                    Some(&leaf.address),
                    leaf.hint.as_deref(),
                )
                .await?
            }
            MergeStrategy::Container => {
                merger::merge_container(
                    &ctx.db,
                    &ctx.vault,
                    template,
                    note_proto.clone(),
                    &p3_out.fields,
                    &p3_out.roster,
                    &body_rendered,
                    &source_id,
                    Some(&leaf.address),
                    leaf.hint.as_deref(),
                )
                .await?
            }
            MergeStrategy::SourceBound => {
                merger::merge_source_bound(
                    &ctx.db,
                    &ctx.vault,
                    note_proto.clone(),
                    &p3_out.fields,
                    &body_rendered,
                    &source_id,
                    prior_source_id.as_deref(),
                    &leaf.address,
                    leaf.hint.as_deref(),
                    &content_hash,
                )
                .await?
            }
            MergeStrategy::TitleAuthor => {
                // title_author deduplicates via match_key (title+author slug);
                // merge behaviour is identical to pure_atomic.
                merger::merge_pure_atomic(
                    &ctx.db,
                    &ctx.vault,
                    note_proto.clone(),
                    &p3_out.fields,
                    &body_rendered,
                    &source_id,
                    Some(&leaf.address),
                    leaf.hint.as_deref(),
                )
                .await?
            }
        };

        // Track results
        let note_id = match &outcome {
            merger::MergeOutcome::Created { note_id, .. } => {
                atomic_notes_created += 1;
                note_id.clone()
            }
            merger::MergeOutcome::FilledFields { note_id, .. }
            | merger::MergeOutcome::AddedRoster { note_id, .. }
            | merger::MergeOutcome::Regenerated { note_id, .. }
            | merger::MergeOutcome::Conflict { note_id, .. }
            | merger::MergeOutcome::Noop { note_id } => {
                atomic_notes_merged += 1;
                note_id.clone()
            }
        };

        leaf_note_ids.insert(leaf.address.clone(), note_id.clone());

        // Find the actual note record for pass 4
        if let Some(actual_note) = db::get_note(&ctx.db, &note_id).await? {
            leaf_notes.push((actual_note, p3_out));
        } else {
            leaf_notes.push((note_proto, p3_out));
        }
    }

    // 10. Derive implicit edges
    let mut edges_created = derive_implicit_edges(
        &ctx.db,
        &outline_note_id,
        &source_id,
        &leaves,
        &leaf_note_ids,
    )
    .await?;

    // 11. Relationship edges: batch path inserts from batch_rel_edges; standard path runs Pass 4
    if ctx.config.pipeline.is_batch() {
        for rel_edge in &batch_rel_edges {
            let src_note = find_note_by_match_key(&ctx.db, &rel_edge.source).await?;
            let tgt_note = find_note_by_match_key(&ctx.db, &rel_edge.target).await?;
            if let (Some(src), Some(tgt)) = (src_note, tgt_note) {
                let edge = EdgeRecord {
                    id: Uuid::new_v4().to_string(),
                    source_note_id: src.id,
                    target_note_id: tgt.id,
                    edge_type: rel_edge.relationship.clone(),
                    why: Some(rel_edge.why.clone()),
                    from_source: source_id.clone(),
                    weight: 1.0,
                    metadata: None,
                    created_at: now_rfc3339(),
                };
                if insert_edge_if_not_exists(&ctx.db, &edge).await? {
                    edges_created += 1;
                }
            }
        }
    } else if !leaf_notes.is_empty() {
        let (nodes_str, toc_str, implicit_str) =
            build_pass4_input(&leaf_notes, &leaves, &outline_note_id);

        let p4_params = Pass4Params {
            nodes: &nodes_str,
            toc: &toc_str,
            implicit_edges: &implicit_str,
        };
        let p4_prompt = prompt::build_pass4(p4_params);
        let mut p4_opts = InferOpts::from_settings(&ctx.config.llm.synthesis);
        p4_opts.json_mode = true;

        let p4_raw = llm.infer(&p4_prompt, p4_opts).await
            .with_context(|| "LLM Pass 4 inference failed")?;

        let cleaned = strip_markdown_fences(&p4_raw);
        if let Ok(edges_val) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            if let Some(arr) = edges_val.as_array() {
                for edge_val in arr {
                    let source_mk = edge_val["source"].as_str().unwrap_or("");
                    let target_mk = edge_val["target"].as_str().unwrap_or("");
                    let rel = edge_val["relationship"].as_str().unwrap_or("");
                    let why = edge_val["why"].as_str().unwrap_or("");

                    if source_mk.is_empty() || target_mk.is_empty() || rel.is_empty() {
                        continue;
                    }

                    let src_note = find_note_by_match_key(&ctx.db, source_mk).await?;
                    let tgt_note = find_note_by_match_key(&ctx.db, target_mk).await?;

                    if let (Some(src), Some(tgt)) = (src_note, tgt_note) {
                        let edge = EdgeRecord {
                            id: Uuid::new_v4().to_string(),
                            source_note_id: src.id,
                            target_note_id: tgt.id,
                            edge_type: rel.to_string(),
                            why: Some(why.to_string()),
                            from_source: source_id.clone(),
                            weight: 1.0,
                            metadata: None,
                            created_at: now_rfc3339(),
                        };
                        if insert_edge_if_not_exists(&ctx.db, &edge).await? {
                            edges_created += 1;
                        }
                    }
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(IngestResult {
        source_id,
        outline_note_id,
        atomic_notes_created,
        atomic_notes_merged,
        edges_created,
        pass1_llm_called,
        duration_ms,
    })
}

fn merge_strategy_str(strategy: &MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::PureAtomic => "pure_atomic",
        MergeStrategy::Container => "container",
        MergeStrategy::SourceBound => "source_bound",
        MergeStrategy::TitleAuthor => "title_author",
    }
}

fn tracing_warn(msg: &str) {
    eprintln!("WARN: {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toc_basic() {
        let toc = "1.1 Ian Kitajima [person] | hint: see attendee list\n\
                   1.2 PICHTR [organization]\n\
                   1.3 Sovereign AI [concept] | context_at: 2.1,2.2\n\
                   1.4 Unknown Entity [?]\n";
        let leaves = parse_toc(toc);
        // [?] should be skipped
        assert_eq!(leaves.len(), 3);
        assert_eq!(leaves[0].name, "Ian Kitajima");
        assert_eq!(leaves[0].entity_type, "person");
        assert_eq!(leaves[0].hint.as_deref(), Some("see attendee list"));
        assert_eq!(leaves[1].name, "PICHTR");
        assert!(leaves[2].context_at.contains(&"2.1".to_string()));
        assert!(leaves[2].context_at.contains(&"2.2".to_string()));
    }

    #[test]
    fn parse_toc_task_assignee() {
        let toc = "2.1 Fix bug \u{2014} Alice [task]\n";
        let leaves = parse_toc(toc);
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].name, "Fix bug");
        assert_eq!(leaves[0].assignee.as_deref(), Some("Alice"));
    }

    #[test]
    fn validate_preprocessed_toc_valid() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let templates = TemplateRegistry::load(&std::path::PathBuf::from(&manifest).join("llm").join("plugins").join("anansi.plugin").join("references").join("templates"))
            .expect("load templates");
        let toc = "1.1 Ian Kitajima [person]\n1.2 PICHTR [organization]\n";
        let result = validate_preprocessed_toc(toc, &templates);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn validate_preprocessed_toc_duplicate_address() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let templates = TemplateRegistry::load(&std::path::PathBuf::from(&manifest).join("llm").join("plugins").join("anansi.plugin").join("references").join("templates"))
            .expect("load templates");
        let toc = "1.1 Ian Kitajima [person]\n1.1 PICHTR [organization]\n";
        let result = validate_preprocessed_toc(toc, &templates);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate address"));
    }

    #[test]
    fn validate_preprocessed_toc_unknown_type() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let templates = TemplateRegistry::load(&std::path::PathBuf::from(&manifest).join("llm").join("plugins").join("anansi.plugin").join("references").join("templates"))
            .expect("load templates");
        let toc = "1.1 Something [nonexistent_type]\n";
        let result = validate_preprocessed_toc(toc, &templates);
        assert!(result.is_err());
    }

    #[test]
    fn parse_pass3_response_valid() {
        let raw = r#"{
            "fields": {"name": "Ian Kitajima", "contact_email": "ian@example.com"},
            "roster": {},
            "lede": "Research director at PICHTR.",
            "why": "Ian Kitajima is a research director at PICHTR focused on AI policy.",
            "tags": ["ai", "research"],
            "entities": [{"name": "PICHTR", "entity_type": "organization", "slug": "pichtr"}]
        }"#;
        let out = parse_pass3_response(raw).unwrap();
        assert_eq!(out.lede, "Research director at PICHTR.");
        assert_eq!(out.fields.get("name").unwrap(), "Ian Kitajima");
        assert_eq!(out.entities.len(), 1);
        assert_eq!(out.entities[0].name, "PICHTR");
    }

    #[test]
    fn parse_pass3_response_with_fences() {
        let raw = "```json\n{\"fields\":{},\"roster\":{},\"lede\":\"Test\",\"why\":\"Test.\",\"tags\":[],\"entities\":[]}\n```";
        let out = parse_pass3_response(raw).unwrap();
        assert_eq!(out.lede, "Test");
    }

    #[test]
    fn parse_simple_yaml_basic() {
        let yaml = "title: My Document\nsource_type: meeting_summary\n";
        let map = parse_simple_yaml(yaml);
        assert_eq!(map.get("title").unwrap(), "My Document");
        assert_eq!(map.get("source_type").unwrap(), "meeting_summary");
    }

    #[test]
    fn sha256_consistent() {
        let h1 = sha256("hello world");
        let h2 = sha256("hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}

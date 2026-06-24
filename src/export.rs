use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use zip::write::SimpleFileOptions;

use crate::db::{self, DbPool, EdgeRecord, NoteRecord};
use crate::wiki::WikiStore;

// ---------------------------------------------------------------------------
// BFS helper — shared by both export types
// ---------------------------------------------------------------------------

pub struct BfsResult {
    pub root: NoteRecord,
    pub nodes: HashMap<String, NoteRecord>, // id -> note (includes root)
    pub edges: Vec<EdgeRecord>,
}

pub async fn bfs(pool: &DbPool, start_id: &str, depth: usize) -> Result<BfsResult> {
    let root = db::get_note(pool, start_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Note not found: {start_id}"))?;

    let mut nodes: HashMap<String, NoteRecord> = HashMap::new();
    let mut all_edges: Vec<EdgeRecord> = Vec::new();
    let mut edge_ids_seen: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![start_id.to_string()];

    nodes.insert(root.id.clone(), root.clone());
    visited.insert(start_id.to_string());

    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut next_frontier: Vec<String> = Vec::new();

        for node_id in &frontier {
            let edges = db::edges_for_note(pool, node_id).await?;
            for edge in edges {
                let neighbour_id = if edge.source_note_id == *node_id {
                    edge.target_note_id.clone()
                } else {
                    edge.source_note_id.clone()
                };

                if edge_ids_seen.insert(edge.id.clone()) {
                    all_edges.push(edge);
                }

                if !visited.contains(&neighbour_id) {
                    visited.insert(neighbour_id.clone());
                    next_frontier.push(neighbour_id.clone());
                    if let Ok(Some(note)) = db::get_note(pool, &neighbour_id).await {
                        nodes.insert(neighbour_id.clone(), note);
                    }
                }
            }
        }

        frontier = next_frontier;
    }

    Ok(BfsResult { root, nodes, edges: all_edges })
}

// ---------------------------------------------------------------------------
// Context export — single markdown document for LLM consumption
// ---------------------------------------------------------------------------

pub async fn export_context(pool: &DbPool, start_id: &str, depth: usize) -> Result<String> {
    let bfs = bfs(pool, start_id, depth).await?;

    let mut out = String::new();
    out.push_str(&format!(
        "# Context: {}\n\n*Root note: `{}` — graph depth: {}*\n\n",
        bfs.root.name, bfs.root.match_key, depth
    ));
    out.push_str(&format!(
        "**{} notes, {} edges in this context.**\n\n---\n\n",
        bfs.nodes.len(),
        bfs.edges.len()
    ));

    // Root note first, then others ordered by name
    let mut note_ids: Vec<&String> = bfs.nodes.keys().collect();
    note_ids.sort_by_key(|id| {
        if *id == &bfs.root.id { "\x00".to_string() } // sorts first
        else { bfs.nodes[*id].name.to_lowercase() }
    });

    for id in &note_ids {
        let note = &bfs.nodes[*id];
        out.push_str(&format!("## {} ({})\n\n", note.name, note.entity_type));
        out.push_str(&format!("**match_key:** `{}`  \n", note.match_key));
        out.push_str(&format!("**id:** `{}`\n\n", note.id));

        if let Some(lede) = &note.lede {
            out.push_str(&format!("**Summary:** {lede}\n\n"));
        }
        if let Some(why) = &note.why {
            out.push_str(&format!("**Why it matters:** {why}\n\n"));
        }
        if let Some(content) = &note.content {
            out.push_str(&format!("{content}\n\n"));
        }

        // Edges from this note
        let note_edges: Vec<&EdgeRecord> = bfs.edges.iter().filter(|e| {
            e.source_note_id == note.id || e.target_note_id == note.id
        }).collect();

        if !note_edges.is_empty() {
            out.push_str("**Connections:**\n");
            for edge in note_edges {
                let other_id = if edge.source_note_id == note.id {
                    &edge.target_note_id
                } else {
                    &edge.source_note_id
                };
                let other_name = bfs.nodes.get(other_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or("(unknown)");
                let direction = if edge.source_note_id == note.id { "→" } else { "←" };
                let why_str = edge.why.as_deref().map(|w| format!(" — {w}")).unwrap_or_default();
                out.push_str(&format!(
                    "- {direction} **{}** `{}`{why_str}\n",
                    other_name, edge.edge_type
                ));
            }
            out.push('\n');
        }

        out.push_str("---\n\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Obsidian vault export — zip of .md files with [[wiki-links]]
// ---------------------------------------------------------------------------

/// Sanitize a note name into a safe filename (no slashes, no special chars)
fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn export_vault(
    pool: &DbPool,
    start_id: &str,
    depth: usize,
    exports_dir: &PathBuf,
) -> Result<String> {
    let bfs = bfs(pool, start_id, depth).await?;

    // Collect edges per note
    let mut edges_by_note: HashMap<String, Vec<&EdgeRecord>> = HashMap::new();
    for edge in &bfs.edges {
        edges_by_note.entry(edge.source_note_id.clone()).or_default().push(edge);
        edges_by_note.entry(edge.target_note_id.clone()).or_default().push(edge);
    }

    // Build zip in memory
    let mut zip_buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for note in bfs.nodes.values() {
            let filename = format!("{}.md", safe_filename(&note.name));
            zip.start_file(&filename, options)?;

            // YAML frontmatter
            let mut content = String::new();
            content.push_str("---\n");
            content.push_str(&format!("id: {}\n", note.id));
            content.push_str(&format!("entity_type: {}\n", note.entity_type));
            content.push_str(&format!("match_key: {}\n", note.match_key));
            if let Some(lede) = &note.lede {
                // Escape quotes in YAML
                content.push_str(&format!("lede: \"{}\"\n", lede.replace('"', "\\\"")));
            }
            content.push_str(&format!("updated_at: {}\n", note.updated_at));
            content.push_str("---\n\n");

            // Body
            content.push_str(&format!("# {}\n\n", note.name));
            if let Some(lede) = &note.lede {
                content.push_str(&format!("{lede}\n\n"));
            }
            if let Some(why) = &note.why {
                content.push_str(&format!("*{why}*\n\n"));
            }
            if let Some(body) = &note.content {
                content.push_str(&format!("{body}\n\n"));
            }

            // Connections as wiki-links
            if let Some(note_edges) = edges_by_note.get(&note.id) {
                content.push_str("## Connections\n\n");
                for edge in note_edges {
                    let other_id = if edge.source_note_id == note.id {
                        &edge.target_note_id
                    } else {
                        &edge.source_note_id
                    };
                    if let Some(other) = bfs.nodes.get(other_id) {
                        let why_str = edge.why.as_deref()
                            .map(|w| format!(" — {w}"))
                            .unwrap_or_default();
                        content.push_str(&format!(
                            "- [[{}]] `{}`{why_str}\n",
                            safe_filename(&other.name),
                            edge.edge_type
                        ));
                    }
                }
                content.push('\n');
            }

            zip.write_all(content.as_bytes())?;
        }

        zip.finish()?;
    }

    // Write to exports dir
    std::fs::create_dir_all(exports_dir)?;
    let export_id = uuid::Uuid::new_v4().to_string();
    let root_slug = safe_filename(&bfs.root.name).to_lowercase().replace(' ', "-");
    let zip_name = format!("vault-{root_slug}-{export_id}.zip");
    let zip_path = exports_dir.join(&zip_name);
    std::fs::write(&zip_path, &zip_buf)?;

    Ok(zip_name)
}

// ---------------------------------------------------------------------------
// Full-wiki export — the complete graph as a downloadable wiki bundle (Build-18)
// ---------------------------------------------------------------------------

/// Project ALL live notes into the canonical LLM-wiki layout (note files +
/// `index.md`) and package it as a zip under `exports_dir`. Returns the zip
/// filename (served by the existing `/exports/{filename}` route). Built into a
/// throwaway temp dir with an uncapped, force-enabled `WikiStore` so the bundle
/// is the FULL graph regardless of the server's `[wiki]` config, and never
/// touches the live wiki dir. Read-only against Postgres.
pub async fn export_wiki(pool: &DbPool, exports_dir: &PathBuf) -> Result<String> {
    std::fs::create_dir_all(exports_dir)?;
    let export_id = uuid::Uuid::new_v4().to_string();
    let build_dir = exports_dir.join(format!("wiki-build-{export_id}"));
    std::fs::create_dir_all(&build_dir)?;

    // Reuse the canonical projection: uncapped (max_*=0 → all live notes),
    // force-enabled, into the temp dir. Same renderer as the server wiki.
    let wiki = WikiStore {
        root: build_dir.clone(),
        enabled: true,
        max_bytes: 0,
        max_notes: 0,
        crawl_enabled: false,
    };
    let build = wiki.crawl(pool).await;

    // Zip whatever was produced even if some notes errored; only a hard failure
    // (e.g. the projection couldn't start) aborts.
    if let Err(e) = build {
        let _ = std::fs::remove_dir_all(&build_dir);
        return Err(e);
    }

    // Don't ship log.md: building via crawl() journals a synthetic "## [date]
    // crawl" line that never happened on the client. The exported bundle is a
    // content snapshot (note files + index.md); the local install keeps its own
    // journal.
    let _ = std::fs::remove_file(build_dir.join("log.md"));

    let zip_name = format!("wiki-{export_id}.zip");
    let zip_path = exports_dir.join(&zip_name);
    let result = zip_dir(&build_dir, &zip_path);
    let _ = std::fs::remove_dir_all(&build_dir); // best-effort cleanup
    result?;

    Ok(zip_name)
}

/// Zip every top-level file in `src_dir` (flat) into `zip_path`.
fn zip_dir(src_dir: &Path, zip_path: &Path) -> Result<()> {
    let mut zip_buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for entry in std::fs::read_dir(src_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let content = std::fs::read(&path)?;
            zip.start_file(&name, options)?;
            zip.write_all(&content)?;
        }
        zip.finish()?;
    }
    std::fs::write(zip_path, &zip_buf)?;
    Ok(())
}

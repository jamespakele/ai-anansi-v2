//! LLM-Wiki dual-write layer (Build-12).
//!
//! A file-native *projection* of the Anansi knowledge graph following the
//! Karpathy LLM-wiki pattern. Postgres remains the source of truth; after a
//! successful capture, the affected notes are materialized from canonical DB
//! rows into markdown files under the configured wiki root:
//!
//! ```text
//! ~/llm-wiki/                 ← the configured [wiki] dir (default ~/llm-wiki)
//!   <slug>.<entity_type>.md   ← one file per note (frontmatter + [[wikilinks]])
//!   index.md                  ← catalog grouped by PARA (Projects, Areas, Resources, Archives)
//!   log.md                    ← append-only ingest journal
//! ```
//!
//! Writes are gated by `[wiki] enabled` (default false) and are strictly
//! non-authoritative and non-fatal: a wiki write failure is logged and never
//! rolls back or fails the underlying DB write.

use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::db::{self, DbPool, EdgeRecord, NoteRecord};

pub struct WikiStore {
    pub root: PathBuf,
    pub enabled: bool,
    /// Tipping-point caps (Build-15). 0 = unlimited.
    pub max_bytes: u64,
    pub max_notes: u64,
    /// Whether the background crawl runs (Build-22). When false, the crawl is NOT
    /// the index author, so capture/refresh must maintain index.md synchronously
    /// even under caps (no crawl ⇒ no eviction ⇒ the all-live index is correct).
    pub crawl_enabled: bool,
}

/// An `index.md` catalog row: (entity_type, name, lede).
type IndexEntry = (String, String, Option<String>);

/// Outcome of one anansi-crawl maintenance pass (Build-13, +evicted in Build-15).
#[derive(Debug, Default, serde::Serialize)]
pub struct CrawlReport {
    pub notes_projected: usize,
    /// Live notes beyond the tipping-point cap whose files were evicted (Build-15).
    pub evicted: usize,
    pub orphans_removed: usize,
    pub errors: usize,
}

impl WikiStore {
    pub fn from_config(config: &Config) -> Self {
        Self {
            root: PathBuf::from(&config.wiki.dir),
            enabled: config.wiki.enabled,
            max_bytes: config.wiki.max_mb.saturating_mul(1_048_576),
            max_notes: config.wiki.max_notes,
            crawl_enabled: config.wiki.crawl_enabled,
        }
    }

    /// True when the background crawl owns `index.md` (resident-only) — i.e. caps
    /// are active AND the crawl actually runs. Only then may the synchronous
    /// capture/refresh paths skip the all-live index rebuild and defer to it.
    fn crawl_owns_index(&self) -> bool {
        self.caps_active() && self.crawl_enabled
    }

    /// Project a set of just-written notes from canonical DB state into wiki
    /// files, then rebuild `index.md` and append to `log.md`. No-op when
    /// disabled. Reads notes/edges back from Postgres so the file reflects the
    /// post-merge canonical content (COALESCE upserts are honored).
    pub async fn materialize(
        &self,
        pool: &DbPool,
        note_ids: &[String],
        source_title: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let written = self.project(pool, note_ids).await?;
        self.append_log(source_title, written)?;
        Ok(())
    }

    /// Write the given notes' files from canonical DB state and rebuild
    /// `index.md`. Shared by `materialize` (capture/ingest) and `crawl` — does
    /// NOT append a log line, so each caller journals in its own voice. Returns
    /// the number of note files written. No-op when disabled.
    async fn project(&self, pool: &DbPool, note_ids: &[String]) -> Result<usize> {
        if !self.enabled {
            return Ok(0);
        }
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("creating wiki root: {}", self.root.display()))?;

        // id -> (name, entity_type) for resolving typed wikilinks on connections.
        let mut names: HashMap<String, (String, String)> = HashMap::new();
        let mut written = 0usize;

        for id in note_ids {
            // Per-note errors are logged and skipped so one bad note can't leave
            // the index un-rebuilt for the rest of the batch.
            let note = match db::get_note(pool, id).await {
                Ok(Some(n)) => n,
                Ok(None) => continue,
                Err(e) => {
                    eprintln!("[wiki] get_note failed for {id}: {e}");
                    continue;
                }
            };
            names.insert(id.clone(), (note.name.clone(), note.entity_type.clone()));

            let edges = match db::edges_for_note(pool, id).await {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[wiki] edges_for_note failed for {id}: {e}");
                    continue;
                }
            };
            // Resolve each connected note's identity for wikilink rendering.
            // Skip archived targets — their files are intentionally not in the
            // live wiki, so a link to them would dangle.
            for e in &edges {
                let other = edge_other(&note.id, e);
                if !names.contains_key(other) {
                    if let Ok(Some(on)) = db::get_note(pool, other).await {
                        if !on.entity_type.starts_with("archive-") {
                            names.insert(other.to_string(), (on.name, on.entity_type));
                        }
                    }
                }
            }

            if let Err(e) = self.write_note(&note, &edges, &names) {
                eprintln!("[wiki] write_note failed for '{}': {e}", note.name);
                continue;
            }
            written += 1;
        }

        // When the crawl owns the index (caps active AND crawl running), it is the
        // sole author of the resident-only index — rebuilding the all-live index
        // here would list evicted notes as broken wikilinks. Otherwise (no caps,
        // or caps but no crawl) keep maintaining index.md synchronously.
        if !self.crawl_owns_index() {
            self.rebuild_index(pool).await?;
        }
        Ok(written)
    }

    /// Whether a tipping-point cap is configured (Build-15). When true, the crawl
    /// owns `index.md` (resident-only); the capture/refresh paths defer to it.
    fn caps_active(&self) -> bool {
        self.max_bytes > 0 || self.max_notes > 0
    }

    /// The anansi-crawl maintenance pass: reconcile the wiki against canonical
    /// PG state (re-project every live note coldest-first, repairing missing /
    /// stale files and rebuilding `index.md`), garbage-collect orphan wiki files
    /// no live note backs, and journal a crawl summary. Idempotent and
    /// self-healing. No-op when disabled.
    pub async fn crawl(&self, pool: &DbPool) -> Result<CrawlReport> {
        let mut report = CrawlReport::default();
        if !self.enabled {
            return Ok(report);
        }
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("creating wiki root: {}", self.root.display()))?;

        // Capture the crawl's start instant: any file written at/after this (by a
        // concurrent capture not yet in our DB snapshot) is spared from GC.
        let crawl_start = std::time::SystemTime::now();

        // 1. Resident-set projection: hottest-first, bounded by the tipping-point
        //    caps. Each projected note accrues toward the byte/count budget; once
        //    a cap is reached every colder note is evicted (its file is swept by
        //    the GC below, since only resident filenames go into `expected`).
        let ids = db::live_note_ids_by_access_desc(pool).await?;

        let mut names: HashMap<String, (String, String)> = HashMap::new();
        let mut resident_files: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut resident_entries: Vec<IndexEntry> = Vec::new();
        let mut bytes = 0u64;
        let mut project_failed = false;

        for id in &ids {
            // Stop projecting once either cap is exhausted (0 = unlimited).
            let over = (self.max_notes > 0 && report.notes_projected as u64 >= self.max_notes)
                || (self.max_bytes > 0 && bytes >= self.max_bytes);
            if over {
                report.evicted += 1;
                continue;
            }

            let note = match db::get_note(pool, id).await {
                Ok(Some(n)) => n,
                Ok(None) => continue,
                Err(e) => {
                    eprintln!("[crawl] get_note failed for {id}: {e}");
                    report.errors += 1;
                    project_failed = true;
                    continue;
                }
            };
            names.insert(id.clone(), (note.name.clone(), note.entity_type.clone()));

            let edges = match db::edges_for_note(pool, id).await {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[crawl] edges_for_note failed for {id}: {e}");
                    report.errors += 1;
                    project_failed = true;
                    continue;
                }
            };
            for e in &edges {
                let other = edge_other(&note.id, e);
                if !names.contains_key(other) {
                    if let Ok(Some(on)) = db::get_note(pool, other).await {
                        if !on.entity_type.starts_with("archive-") {
                            names.insert(other.to_string(), (on.name, on.entity_type));
                        }
                    }
                }
            }

            if let Err(e) = self.write_note(&note, &edges, &names) {
                eprintln!("[crawl] write_note failed for '{}': {e}", note.name);
                report.errors += 1;
                // Don't GC this pass: the note's existing (valid) file from a
                // prior crawl must not be deleted over a transient write hiccup.
                project_failed = true;
                continue;
            }
            let fname = expected_filename(&note.entity_type, &note.name);
            bytes += std::fs::metadata(self.root.join(&fname))
                .map(|m| m.len())
                .unwrap_or(0);
            resident_files.insert(fname);
            resident_entries.push((
                note.entity_type.clone(),
                note.name.clone(),
                note.lede.clone(),
            ));
            report.notes_projected += 1;
        }

        // 2 & 3. Author index.md and GC non-resident files.
        //
        //  • Uncapped (default): the live note set IS the resident set, so derive
        //    the GC keep-set AND the index authoritatively from all_note_summaries
        //    — independent of whether any single note's projection errored this
        //    pass. A single bad row therefore can NOT freeze the index or GC
        //    (the erroring note just isn't refreshed this pass; it stays kept).
        //  • Capped: residency was decided by the hottest-first loop above, so we
        //    must use the loop's resident set and skip both on a projection error
        //    (conservative — don't delete/drop while the resident set is uncertain).
        let mut do_maintenance = true;
        if self.caps_active() {
            if project_failed {
                do_maintenance = false;
            }
        } else {
            match db::all_note_summaries(pool).await {
                Ok(summaries) => {
                    resident_files.clear();
                    resident_entries.clear();
                    for s in &summaries {
                        resident_files.insert(expected_filename(&s.entity_type, &s.name));
                        resident_entries.push((
                            s.entity_type.clone(),
                            s.name.clone(),
                            s.lede.clone(),
                        ));
                    }
                }
                Err(e) => {
                    eprintln!("[crawl] all_note_summaries failed — skipping GC/index: {e}");
                    report.errors += 1;
                    do_maintenance = false;
                }
            }
        }

        if do_maintenance {
            if let Err(e) = self.write_index(&resident_entries) {
                eprintln!("[crawl] index rebuild failed: {e}");
                report.errors += 1;
            }

            resident_files.insert("index.md".to_string());
            resident_files.insert("log.md".to_string());
            resident_files.insert("lint.md".to_string()); // Build-14 lint report
            match std::fs::read_dir(&self.root) {
                Err(e) => {
                    // Non-fatal: skip GC this pass, still journal below.
                    eprintln!("[crawl] reading wiki root failed — skipping GC: {e}");
                    report.errors += 1;
                }
                Ok(dir) => {
                    for entry in dir {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(_) => continue,
                        };
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        let fname = match path.file_name().and_then(|s| s.to_str()) {
                            Some(f) => f.to_string(),
                            None => continue,
                        };
                        if resident_files.contains(&fname) {
                            continue;
                        }
                        if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
                            if mtime >= crawl_start {
                                continue;
                            }
                        }
                        match std::fs::remove_file(&path) {
                            Ok(()) => report.orphans_removed += 1,
                            Err(e) => {
                                eprintln!("[crawl] failed to remove {}: {e}", path.display());
                                report.errors += 1;
                            }
                        }
                    }
                }
            }
        }

        // 4. Journal the crawl.
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let line = format!(
            "## [{date}] crawl | {} resident, {} evicted, {} removed\n",
            report.notes_projected, report.evicted, report.orphans_removed
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("log.md"))
        {
            let _ = f.write_all(line.as_bytes());
        }

        Ok(report)
    }

    /// Pure resident-set cutoff: number of leading (hottest-first) notes that fit
    /// under the caps. 0 caps = unlimited. Build-15.
    #[cfg(test)]
    fn resident_cutoff(sizes: &[u64], max_bytes: u64, max_notes: u64) -> usize {
        let mut bytes = 0u64;
        let mut n = 0usize;
        for &sz in sizes {
            if (max_notes > 0 && n as u64 >= max_notes) || (max_bytes > 0 && bytes >= max_bytes) {
                break;
            }
            bytes += sz;
            n += 1;
        }
        n
    }

    /// Rebuild `index.md` from current canonical state. Used by removal paths
    /// (delete/archive) that drop a file but write no note, so the catalog stays
    /// consistent. No-op when disabled.
    pub async fn refresh_index(&self, pool: &DbPool) -> Result<()> {
        if !self.enabled || self.crawl_owns_index() {
            // The crawl owns the resident-only index; an all-live rebuild here
            // would re-introduce evicted notes. The next crawl reconciles. (When
            // caps are set but the crawl is off, we DO rebuild — nothing evicts.)
            return Ok(());
        }
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("creating wiki root: {}", self.root.display()))?;
        self.rebuild_index(pool).await
    }

    /// Render and atomically write a single note's wiki file. No-op when disabled.
    pub fn write_note(
        &self,
        note: &NoteRecord,
        edges: &[EdgeRecord],
        names: &HashMap<String, (String, String)>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.note_path(&note.entity_type, &note.name);
        let content = render_note_markdown(note, edges, names);
        atomic_write(&path, &content)
    }

    /// Best-effort removal of a note's wiki file (on delete/archive). No-op when
    /// disabled; a missing file is not an error.
    pub fn remove_note(&self, entity_type: &str, name: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.note_path(entity_type, name);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("removing wiki note: {}", path.display())),
        }
    }

    fn note_path(&self, entity_type: &str, name: &str) -> PathBuf {
        self.root.join(expected_filename(entity_type, name))
    }

    /// Rebuild `index.md` from ALL live notes (capture / delete / archive paths).
    /// The Build-15 crawl instead calls `write_index` with the resident set only.
    async fn rebuild_index(&self, pool: &DbPool) -> Result<()> {
        let entries: Vec<IndexEntry> = db::all_note_summaries(pool)
            .await?
            .into_iter()
            .map(|s| (s.entity_type, s.name, s.lede))
            .collect();
        self.write_index(&entries)
    }

    /// Write `index.md` from an explicit entry set, grouped by PARA section
    /// (Projects, Areas, Resources, Archives). Within Resources, entries are
    /// grouped by entity_type. Callers decide the membership: all-live or resident.
    fn write_index(&self, entries: &[IndexEntry]) -> Result<()> {
        let mut by_type: BTreeMap<&str, Vec<&IndexEntry>> = BTreeMap::new();
        for e in entries {
            by_type.entry(e.0.as_str()).or_default().push(e);
        }

        // PARA section order + entity_type grouping within Resources.
        // Returns (section_number, section_label).
        let para_section = |etype: &str| -> (u8, &'static str) {
            match etype {
                "project" => (1, "Projects"),
                "area" => (2, "Areas"),
                et if et.starts_with("archive-") => (4, "Archives"),
                // Everything else is Resources (PARA §3).
                _ => (3, "Resources"),
            }
        };

        // Sort entries by PARA section, then by entity_type within Resources.
        let mut sorted: Vec<(&str, &Vec<&IndexEntry>)> =
            by_type.iter().map(|(k, v)| (*k, v)).collect();
        sorted.sort_by_key(|(et, _)| para_section(et));

        let mut out = String::new();
        out.push_str("# Anansi LLM-Wiki — Index\n\n");
        out.push_str(&format!(
            "Catalog of {} notes, organized by PARA (Projects, Areas, Resources, Archives).\n\n",
            entries.len()
        ));

        let mut current_section: u8 = 0;
        for (etype, notes) in &sorted {
            let (section, label) = para_section(etype);

            // Emit section heading when we enter a new PARA section.
            if section != current_section {
                current_section = section;
                out.push_str(&format!("## {section}. {label}\n\n"));
            }

            // Within Resources, emit a subheading per entity_type.
            if section == 3 {
                out.push_str(&format!("### {etype}\n\n"));
            }

            for (et, name, lede) in *notes {
                let link = wikilink(et, name);
                match lede.as_deref().map(oneline) {
                    Some(l) if !l.is_empty() => out.push_str(&format!("- {link} — {l}\n")),
                    _ => out.push_str(&format!("- {link}\n")),
                }
            }
            out.push('\n');
        }

        atomic_write(&self.root.join("index.md"), &out)
    }

    /// Append one parseable journal line per ingest to `log.md`.
    fn append_log(&self, source_title: &str, n_notes: usize) -> Result<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let line = format!(
            "## [{date}] ingest | {} (+{n_notes} notes)\n",
            oneline(source_title)
        );
        let log_path = self.root.join("log.md");
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening log: {}", log_path.display()))?;
        f.write_all(line.as_bytes())
            .with_context(|| format!("appending log: {}", log_path.display()))?;
        Ok(())
    }
}

// ───────────────────────────── rendering ──────────────────────────────────────

/// Render a note's markdown file (frontmatter + body + `## Connections`).
/// Pure — no I/O — so it is directly unit-testable. Mirrors `export.rs`.
fn render_note_markdown(
    note: &NoteRecord,
    edges: &[EdgeRecord],
    names: &HashMap<String, (String, String)>,
) -> String {
    let mut out = String::new();

    // YAML frontmatter
    out.push_str("---\n");
    out.push_str(&format!("anansi_id: {}\n", yaml_value(&note.id)));
    out.push_str(&format!("entity_type: {}\n", yaml_value(&note.entity_type)));
    out.push_str(&format!("name: {}\n", yaml_value(&note.name)));
    out.push_str(&format!("match_key: {}\n", yaml_value(&note.match_key)));
    out.push_str(&format!("updated_at: {}\n", yaml_value(&note.updated_at)));
    out.push_str("---\n\n");

    // Body
    out.push_str(&format!("# {}\n\n", note.name));
    if let Some(lede) = note.lede.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("{lede}\n\n"));
    }
    if let Some(why) = note.why.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("*{why}*\n\n"));
    }
    if let Some(body) = note.content.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("{body}\n\n"));
    }

    // Connections as typed wikilinks
    if !edges.is_empty() {
        out.push_str("## Connections\n\n");
        for e in edges {
            let other = edge_other(&note.id, e);
            if let Some((name, etype)) = names.get(other) {
                let link = wikilink(etype, name);
                let why = e
                    .why
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|w| format!(" — {}", oneline(w)))
                    .unwrap_or_default();
                out.push_str(&format!("- {link} `{}`{why}\n", e.edge_type));
            }
        }
        out.push('\n');
    }

    out
}

// ───────────────────────────── helpers ────────────────────────────────────────

/// The note id on the other end of an edge from `id`'s perspective.
fn edge_other<'a>(id: &str, e: &'a EdgeRecord) -> &'a str {
    if e.source_note_id == id {
        &e.target_note_id
    } else {
        &e.source_note_id
    }
}

/// The wiki filename a live note maps to — must stay in lockstep with
/// `WikiStore::note_path` so crawl orphan-GC doesn't delete real note files.
fn expected_filename(entity_type: &str, name: &str) -> String {
    let slug = slug_name(name);
    let ext = if entity_type.is_empty() {
        "note"
    } else {
        entity_type
    };
    format!("{slug}.{ext}.md")
}

/// Obsidian typed wikilink — matches `vault.rs::wikilink`.
fn wikilink(entity_type: &str, name: &str) -> String {
    let slug = slug_name(name);
    let ext = if entity_type.is_empty() {
        "note"
    } else {
        entity_type
    };
    format!("[[{slug}.{ext}|{name}]]")
}

/// Slug normalization — identical algorithm to `vault.rs::slug_name` /
/// `db::match_key` (lowercase, non-alphanumeric → space, collapse, hyphen-join).
fn slug_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Collapse newlines/tabs to single spaces for one-line contexts (index, log, edge why).
fn oneline(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// YAML-safe scalar — quotes when the value could be misparsed. Matches
/// `writer.rs::yaml_value`.
fn yaml_value(v: &str) -> String {
    let needs_quote = v.contains(':')
        || v.contains('#')
        || v.contains('\n')
        || v.contains('\r')
        || v.contains('\t')
        || v.starts_with(' ')
        || v.ends_with(' ')
        || v.is_empty();
    if needs_quote {
        let escaped = v
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{escaped}\"")
    } else {
        v.to_string()
    }
}

/// Write to a unique tmp file then rename for atomicity. Unlike a fixed `.tmp`
/// suffix, the per-call `<pid>.<seq>` makes concurrent writers (queue watcher +
/// MCP handlers) stage to distinct tmp files, so the final atomic rename yields
/// a complete file (last-writer-wins) instead of interleaved/corrupt content —
/// critical for `index.md`, which every materialize rewrites.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

    // Skip identical rewrites — avoids churn (mtime bumps, IO, backup-dedup
    // defeat) when a full crawl re-projects unchanged notes and index.md.
    if let Ok(existing) = std::fs::read_to_string(path) {
        if existing == content {
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory: {}", parent.display()))?;
    }

    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("wiki");
    let tmp_name = format!(".{fname}.{}.{seq}.tmp", std::process::id());
    let tmp = match path.parent() {
        Some(p) => p.join(tmp_name),
        None => PathBuf::from(tmp_name),
    };

    std::fs::write(&tmp, content)
        .with_context(|| format!("writing tmp file: {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} to {}", tmp.display(), path.display()))?;
    Ok(())
}

// ───────────────────────────── tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, etype: &str, name: &str) -> NoteRecord {
        NoteRecord {
            id: id.to_string(),
            entity_type: etype.to_string(),
            name: name.to_string(),
            match_key: db::match_key(name, etype),
            lede: Some("A short lede.".to_string()),
            why: Some("why it matters".to_string()),
            content: Some("Body content.".to_string()),
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "entity".to_string(),
            created_from: "src".to_string(),
            source_count: 1,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            updated_at: "2026-06-24T00:00:00Z".to_string(),
        }
    }

    fn edge(src: &str, tgt: &str, etype: &str) -> EdgeRecord {
        EdgeRecord {
            id: "e".to_string(),
            source_note_id: src.to_string(),
            target_note_id: tgt.to_string(),
            edge_type: etype.to_string(),
            why: Some("connected".to_string()),
            from_source: "src".to_string(),
            weight: 1.0,
            metadata: None,
            created_at: "2026-06-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_produces_frontmatter_and_typed_wikilink() {
        let n = note("a", "person", "Ian Kitajima");
        let edges = vec![edge("a", "b", "works_at")];
        let mut names = HashMap::new();
        names.insert(
            "b".to_string(),
            ("PICHTR".to_string(), "organization".to_string()),
        );

        let md = render_note_markdown(&n, &edges, &names);

        assert!(md.starts_with("---\n"), "has frontmatter");
        assert!(md.contains("anansi_id: a\n"));
        assert!(md.contains("entity_type: person\n"));
        // match_key contains a colon → yaml_value quotes it.
        assert!(md.contains("match_key: \"person:ian-kitajima\"\n"));
        assert!(md.contains("# Ian Kitajima\n"));
        assert!(md.contains("## Connections"));
        assert!(
            md.contains("- [[pichtr.organization|PICHTR]] `works_at` — connected"),
            "typed wikilink with edge type + why: {md}"
        );
    }

    #[test]
    fn render_omits_connections_when_no_resolved_targets() {
        let n = note("a", "concept", "Sovereign AI");
        // Edge target id "z" is absent from names → not rendered.
        let edges = vec![edge("a", "z", "relates_to")];
        let md = render_note_markdown(&n, &edges, &HashMap::new());
        assert!(!md.contains("z"), "unresolved target id leaks: {md}");
    }

    #[test]
    fn yaml_value_quotes_unsafe_and_passes_plain() {
        assert!(yaml_value("foo: bar").starts_with('"'), "colon quoted");
        assert!(yaml_value("a # b").starts_with('"'), "hash quoted");
        assert_eq!(yaml_value("Ian Kitajima"), "Ian Kitajima");
    }

    #[test]
    fn expected_filename_matches_note_path_basename() {
        // Crawl orphan-GC compares basenames against expected_filename; if these
        // ever diverge from note_path, the crawl would delete live note files.
        let store = WikiStore {
            root: PathBuf::from("/wiki"),
            enabled: true,
            max_bytes: 0,
            max_notes: 0,
            crawl_enabled: false,
        };
        for (etype, name) in [
            ("person", "Ian Kitajima"),
            ("", "Fallback"),
            ("concept", "Sovereign AI"),
        ] {
            let path = store.note_path(etype, name);
            let basename = path.file_name().unwrap().to_str().unwrap();
            assert_eq!(basename, expected_filename(etype, name));
        }
    }

    #[test]
    fn resident_cutoff_respects_caps() {
        let sizes = [10u64, 10, 10, 10, 10]; // 50 bytes total
                                             // Unlimited → all resident.
        assert_eq!(WikiStore::resident_cutoff(&sizes, 0, 0), 5);
        // Count cap.
        assert_eq!(WikiStore::resident_cutoff(&sizes, 0, 3), 3);
        // Size cap: stop once accumulated >= 25 (after 3rd note bytes=30 ≥ 25 → 4th blocked).
        assert_eq!(WikiStore::resident_cutoff(&sizes, 25, 0), 3);
        // Whichever first: count=2 beats size=100.
        assert_eq!(WikiStore::resident_cutoff(&sizes, 100, 2), 2);
        // Empty input.
        assert_eq!(WikiStore::resident_cutoff(&[], 100, 100), 0);
    }

    #[test]
    fn slug_matches_match_key_convention() {
        assert_eq!(slug_name("Ian Kitajima"), "ian-kitajima");
        assert_eq!(slug_name("Sovereign AI!"), "sovereign-ai");
    }

    #[test]
    fn disabled_store_is_a_noop() {
        let store = WikiStore {
            root: PathBuf::from("/definitely/not/writable/xyz"),
            enabled: false,
            max_bytes: 0,
            max_notes: 0,
            crawl_enabled: false,
        };
        let n = note("a", "person", "Nobody");
        // Neither call should touch the filesystem or error.
        assert!(store.write_note(&n, &[], &HashMap::new()).is_ok());
        assert!(store.remove_note("person", "Nobody").is_ok());
    }

    #[test]
    fn remove_missing_file_is_silent() {
        let dir = std::env::temp_dir().join(format!("anansi-wiki-test-{}", std::process::id()));
        let store = WikiStore {
            root: dir,
            enabled: true,
            max_bytes: 0,
            max_notes: 0,
            crawl_enabled: false,
        };
        // No file written yet → removal is a silent success.
        assert!(store.remove_note("person", "Ghost").is_ok());
    }

    #[test]
    fn write_then_path_roundtrip() {
        let dir = std::env::temp_dir().join(format!("anansi-wiki-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = WikiStore {
            root: dir.clone(),
            enabled: true,
            max_bytes: 0,
            max_notes: 0,
            crawl_enabled: false,
        };
        let n = note("a", "person", "Ian Kitajima");

        store.write_note(&n, &[], &HashMap::new()).unwrap();
        let path = dir.join("ian-kitajima.person.md");
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("# Ian Kitajima"));

        store.remove_note("person", "Ian Kitajima").unwrap();
        assert!(!path.exists(), "file should be gone after remove");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

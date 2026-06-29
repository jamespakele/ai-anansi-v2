//! Web Tender (Build-10): background maintenance crawler for the knowledge graph.
//!
//! Periodically checks the graph for integrity issues — dangling edges, duplicate
//! notes, broken wikilinks, circular references, type consistency, and more.
//! In dry-run mode (default), findings are logged but not acted upon. In apply
//! mode, issues are auto-resolved and/or flagged to the `tender_queue` table for
//! review. Mirrors the `crawl.rs` / `inbox.rs` / `queue.rs` watcher pattern.
//! Spawned only when `tender.enabled = true` in anansi.toml.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::config::Config;
use crate::db::{self, DbPool};
use crate::rules::RuleRegistry;

/// Serializes tender passes so concurrent invocations (watcher + MCP tool)
/// never run on top of each other.
static TENDER_LOCK: Mutex<()> = Mutex::const_new(());

/// Maximum pagination iterations per check. Guards against infinite loops if
/// a row persistently fails to delete and the offset never advances.
const MAX_ITERATIONS: u32 = 1000;

/// Insert a tender flag, or update the timestamp of an existing open flag with
/// the same category and match_key.
async fn insert_or_update_flag(
    pool: &DbPool,
    category: &str,
    severity: &str,
    match_key: Option<&str>,
    related_keys: Option<&[String]>,
    description: Option<&str>,
    confidence: Option<f32>,
) -> Result<bool> {
    // Check for an existing open flag with the same category and match_key
    if let Some(existing) = db::find_open_flag(pool, category, match_key).await? {
        db::update_tender_flag_timestamp(pool, &existing.id).await?;
        return Ok(false); // not a new insertion
    }
    db::insert_tender_flag(
        pool,
        category,
        severity,
        match_key,
        related_keys,
        description,
        confidence,
    )
    .await?;
    Ok(true) // new insertion
}

/// Insert an auto-fix audit trail: create a flag with severity=info, then
/// immediately resolve it with resolved_by='auto'. Does not clutter the open queue.
async fn audit_auto_fix(
    pool: &DbPool,
    category: &str,
    match_key: Option<&str>,
    related_keys: Option<&[String]>,
    description: &str,
) {
    if let Ok(flag) = db::insert_tender_flag(
        pool,
        category,
        "info",
        match_key,
        related_keys,
        Some(description),
        Some(1.0),
    )
    .await
    {
        if let Err(e) = db::resolve_auto_flag(pool, &flag.id).await {
            eprintln!(
                "[tender]   failed to auto-resolve audit flag {}: {e}",
                flag.id
            );
        }
    }
}

// ─── Watcher ───────────────────────────────────────────────────────────────────

pub async fn run_tender_watcher(config: Arc<Config>, pool: DbPool, rules: Arc<RuleRegistry>) {
    let interval = std::time::Duration::from_secs(config.tender.interval_secs.max(1));

    eprintln!(
        "[tender] watcher started — every {}s (dry_run: {}, batch_size: {})",
        config.tender.interval_secs, config.tender.dry_run, config.tender.batch_size
    );

    loop {
        match run_tender_pass(&config, &pool, &rules).await {
            Ok(report) => eprintln!(
                "[tender] pass done — {} notes processed, {} edges removed, \
                 {} duplicates merged, {} wikilinks fixed, {} flags inserted, {} errors",
                report.notes_processed,
                report.edges_removed,
                report.duplicates_merged,
                report.wikilinks_fixed,
                report.flags_inserted,
                report.errors,
            ),
            Err(e) => eprintln!("[tender] pass failed: {e}"),
        }

        sleep(interval).await;
    }
}

// ─── Report ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Serialize)]
pub struct TenderReport {
    pub notes_processed: i64,
    pub edges_removed: i64,
    pub duplicates_merged: i64,
    pub wikilinks_fixed: i64,
    pub flags_inserted: i64,
    pub errors: i64,
}

// ─── Pass ───────────────────────────────────────────────────────────────────────

pub async fn run_tender_pass(
    config: &Config,
    pool: &DbPool,
    rules: &RuleRegistry,
) -> Result<TenderReport> {
    let _guard = TENDER_LOCK.lock().await;
    let mut report = TenderReport::default();
    let dry_run = config.tender.dry_run;
    let batch_size = config.tender.batch_size as i64;

    // ── 1. Edge integrity: find and remove dangling edges ──────────────────────
    match check_edge_integrity(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.edges_removed += count;
            eprintln!("[tender] edge integrity: {count} dangling edges handled");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] edge integrity failed: {e}");
        }
    }

    // ── 2. Duplicate edges: find and remove exact duplicates ──────────────────
    match check_duplicate_edges(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.edges_removed += count;
            eprintln!("[tender] duplicate edges: {count} duplicates removed");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] duplicate edges failed: {e}");
        }
    }

    // ── 3. Conflicting edges: flag contradictory edge pairs ───────────────────
    match check_conflicting_edges(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.flags_inserted += count;
            eprintln!("[tender] conflicting edges: {count} conflicts flagged");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] conflicting edges failed: {e}");
        }
    }

    // ── 4. Orphan notes: find notes with no edges ────────────────────────────
    match check_orphan_notes(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.notes_processed += count;
            eprintln!("[tender] orphan notes: {count} orphans found");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] orphan notes failed: {e}");
        }
    }

    // ── 5. Stale notes: find empty notes ─────────────────────────────────────
    match check_stale_notes(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.notes_processed += count;
            eprintln!("[tender] stale notes: {count} stale notes found");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] stale notes failed: {e}");
        }
    }

    // ── 6. Exact duplicates: find and merge duplicate notes ──────────────────────
    match check_exact_duplicates(pool, dry_run, batch_size, rules).await {
        Ok(count) => {
            report.duplicates_merged += count;
            eprintln!("[tender] exact duplicates: {count} duplicates merged");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] exact duplicates failed: {e}");
        }
    }

    // ── 7. Wikilink validity: scan content for broken [[wikilinks]] ──────────
    match check_wikilink_validity(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.wikilinks_fixed += count;
            eprintln!("[tender] wikilink validity: {count} broken links fixed");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] wikilink validity failed: {e}");
        }
    }

    // ── 8. Circular refs: detect edge cycles ─────────────────────────────────
    match check_circular_refs(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.flags_inserted += count;
            eprintln!("[tender] circular refs: {count} cycles flagged");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] circular refs failed: {e}");
        }
    }

    // ── 9. Type consistency: validate edge types match entity types ──────────
    match check_type_consistency(pool, dry_run, batch_size).await {
        Ok(count) => {
            report.flags_inserted += count;
            eprintln!("[tender] type consistency: {count} violations flagged");
        }
        Err(e) => {
            report.errors += 1;
            eprintln!("[tender] type consistency failed: {e}");
        }
    }

    // ── Persist run summary to tender_log ────────────────────────────────────
    if let Err(e) = db::insert_tender_log(
        pool,
        report.notes_processed,
        report.edges_removed,
        report.duplicates_merged,
        report.wikilinks_fixed,
        report.flags_inserted,
        report.errors,
        dry_run,
    )
    .await
    {
        eprintln!("[tender]   failed to write tender_log: {e}");
    }

    Ok(report)
}

// ─── Check Functions ───────────────────────────────────────────────────────────

/// Find and remove dangling edges (edges whose source or target note is gone).
async fn check_edge_integrity(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;
    let mut iterations = 0u32;

    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            eprintln!(
                "[tender]   check_edge_integrity hit max_iterations guard ({}); stopping",
                MAX_ITERATIONS
            );
            break;
        }
        let dangling = db::get_dangling_edges(pool, batch_size, offset).await?;
        if dangling.is_empty() {
            break;
        }
        let count = dangling.len() as i64;

        if dry_run {
            eprintln!("[tender]   would remove {count} dangling edge(s) (offset {offset})");
            total += count;
            offset += batch_size;
            continue;
        }

        // apply mode — deleted rows vacate their slots, so keep offset=0
        // and the next query surfaces the next batch of remaining rows.
        for edge in &dangling {
            if let Err(e) = db::remove_dangling_edge(pool, &edge.id).await {
                eprintln!("[tender]   failed to remove dangling edge {}: {e}", edge.id);
            } else {
                audit_auto_fix(
                    pool,
                    "broken_edge",
                    Some(&edge.source_note_id),
                    Some(&[edge.target_note_id.clone()]),
                    &format!("Dangling edge {} auto-removed", edge.id),
                )
                .await;
            }
        }
        total += count;
        // NO offset += batch_size — deleted rows shift remaining rows up.
    }

    Ok(total)
}

/// Find and remove exact duplicate edges (same source, target, type).
async fn check_duplicate_edges(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;
    let mut iterations = 0u32;

    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            eprintln!(
                "[tender]   check_duplicate_edges hit max_iterations guard ({}); stopping",
                MAX_ITERATIONS
            );
            break;
        }
        let dupes = db::get_duplicate_edges(pool, batch_size, offset).await?;
        if dupes.is_empty() {
            break;
        }

        if dry_run {
            let batch_count: i64 = dupes.iter().map(|d| d.count.unwrap_or(0) - 1).sum();
            eprintln!(
                "[tender]   would remove {batch_count} duplicate edge(s) across {} group(s) (offset {offset})",
                dupes.len()
            );
            total += batch_count;
            offset += batch_size;
            continue;
        }

        for group in &dupes {
            let edges = db::get_edges_for_group(
                pool,
                &group.source_note_id,
                &group.target_note_id,
                &group.edge_type,
            )
            .await?;

            // Skip the first (lowest id), remove the rest
            for edge in edges.iter().skip(1) {
                if let Err(e) = db::remove_duplicate_edge(pool, &edge.id).await {
                    eprintln!(
                        "[tender]   failed to remove duplicate edge {}: {e}",
                        edge.id
                    );
                } else {
                    total += 1;
                    audit_auto_fix(
                        pool,
                        "duplicate_edge",
                        Some(&edge.source_note_id),
                        Some(&[edge.target_note_id.clone(), edge.edge_type.clone()]),
                        &format!("Duplicate edge {} auto-removed", edge.id),
                    )
                    .await;
                }
            }
        }
        // apply mode — removed rows vacate slots; keep offset=0.
    }

    Ok(total)
}

/// Find conflicting edges (contradictory types between same pair) and flag them.
async fn check_conflicting_edges(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;

    loop {
        let conflicts = db::get_conflicting_edges(pool, batch_size, offset).await?;
        if conflicts.is_empty() {
            break;
        }

        if dry_run {
            eprintln!(
                "[tender]   would flag {} conflicting edge(s) (offset {offset})",
                conflicts.len()
            );
            total += conflicts.len() as i64;
            offset += batch_size;
            continue;
        }

        for edge in &conflicts {
            let related = vec![edge.source_note_id.clone(), edge.target_note_id.clone()];
            match insert_or_update_flag(
                pool,
                "conflicting_edges",
                "medium",
                None,
                Some(&related),
                Some(&format!(
                    "Conflicting edge {} between notes {} and {}",
                    edge.edge_type, edge.source_note_id, edge.target_note_id
                )),
                Some(0.8),
            )
            .await
            {
                Ok(is_new) => {
                    if is_new {
                        total += 1;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[tender]   failed to flag conflicting edge {}: {e}",
                        edge.id
                    );
                }
            }
        }
        offset += batch_size;
    }

    Ok(total)
}

/// Find notes with no edges and flag them.
async fn check_orphan_notes(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;

    loop {
        let orphans = db::get_orphan_notes(pool, batch_size, offset).await?;
        if orphans.is_empty() {
            break;
        }

        if dry_run {
            eprintln!(
                "[tender]   would flag {} orphan note(s) (offset {offset})",
                orphans.len()
            );
            total += orphans.len() as i64;
            offset += batch_size;
            continue;
        }

        for note in &orphans {
            match insert_or_update_flag(
                pool,
                "orphan",
                "low",
                Some(&note.match_key),
                None,
                Some(&format!(
                    "Orphan note '{}' ({}) has no edges",
                    note.name, note.match_key
                )),
                Some(0.5),
            )
            .await
            {
                Ok(is_new) => {
                    if is_new {
                        total += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[tender]   failed to flag orphan note {}: {e}", note.id);
                }
            }
        }
        offset += batch_size;
    }

    Ok(total)
}

/// Find notes with empty why and content and flag them.
async fn check_stale_notes(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;

    loop {
        let stale = db::get_stale_notes(pool, batch_size, offset).await?;
        if stale.is_empty() {
            break;
        }

        if dry_run {
            eprintln!(
                "[tender]   would flag {} stale note(s) (offset {offset})",
                stale.len()
            );
            total += stale.len() as i64;
            offset += batch_size;
            continue;
        }

        for note in &stale {
            match insert_or_update_flag(
                pool,
                "stale",
                "low",
                Some(&note.match_key),
                None,
                Some(&format!(
                    "Stale note '{}' ({}) has empty why and content",
                    note.name, note.match_key
                )),
                Some(0.9),
            )
            .await
            {
                Ok(is_new) => {
                    if is_new {
                        total += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[tender]   failed to flag stale note {}: {e}", note.id);
                }
            }
        }
        offset += batch_size;
    }

    Ok(total)
}

/// Find and merge exact duplicate notes (same match_key and entity_type).
///
/// `rules` is threaded in so deduplication thresholds can be overridden via
/// the rule registry. If a `deduplication` rule body is present it is logged;
/// the actual merge currently still uses the existing `COUNT > 1` threshold
/// surfaced by `get_exact_duplicates`. When no rule is found, behavior is
/// unchanged.
async fn check_exact_duplicates(
    pool: &DbPool,
    dry_run: bool,
    batch_size: i64,
    rules: &RuleRegistry,
) -> Result<i64> {
    let dupes = db::get_exact_duplicates(pool).await?;
    if dupes.is_empty() {
        return Ok(0);
    }

    // Rules are accessible here. A `deduplication` rule (if present) could
    // raise the threshold above the default of 1. For now we only surface it
    // so the plumbing is verifiable; the SQL keeps `HAVING COUNT(*) > 1`.
    let dedup_rule = rules.get("deduplication");
    if let Some(body) = dedup_rule {
        eprintln!(
            "[tender]   deduplication rule present ({} bytes), using default threshold",
            body.len()
        );
    }

    if dry_run {
        let total: i64 = dupes.iter().map(|d| d.count.unwrap_or(0) - 1).sum();
        eprintln!(
            "[tender]   would merge {total} duplicate note(s) across {} group(s)",
            dupes.len()
        );
        return Ok(total);
    }

    let mut merged = 0i64;
    for group in &dupes {
        if merged >= batch_size {
            eprintln!(
                "[tender]   reached batch_size limit ({batch_size}), remaining groups deferred"
            );
            break;
        }

        // Fetch all notes in this group, keep the one with the lowest id
        let notes =
            db::get_notes_for_group(pool, &group.normalized_name, &group.entity_type).await?;

        if notes.len() < 2 {
            continue;
        }

        let keep = &notes[0];
        for note in notes.iter().skip(1) {
            if merged >= batch_size {
                break;
            }
            if let Err(e) = db::merge_duplicate_notes(pool, &keep.id, &note.id).await {
                eprintln!(
                    "[tender]   failed to merge duplicate note {} into {}: {e}",
                    note.id, keep.id
                );
            } else {
                merged += 1;
                audit_auto_fix(
                    pool,
                    "dedup",
                    Some(&keep.match_key),
                    Some(&[note.match_key.clone()]),
                    &format!("Merged duplicate note {} into {}", note.id, keep.id),
                )
                .await;
            }
        }
    }

    Ok(merged)
}

/// Scan note content for [[wikilinks]] and verify the targets exist.
async fn check_wikilink_validity(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    use regex::Regex;

    static WIKILINK_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let wikilink_re = WIKILINK_RE.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap());
    let mut total_fixed = 0i64;
    let mut offset = 0i64;
    let mut iterations = 0u32;

    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            eprintln!(
                "[tender]   check_wikilink_validity hit max_iterations guard ({}); stopping",
                MAX_ITERATIONS
            );
            break;
        }
        let batch = db::get_notes_batch(pool, batch_size as u64, offset as u64).await?;
        if batch.is_empty() {
            break;
        }

        for note in &batch {
            let content = match &note.content {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };

            let mut broken_targets: Vec<String> = Vec::new();
            for cap in wikilink_re.captures_iter(content) {
                let target = cap.get(1).unwrap().as_str();
                // Compute the slug (lowercased, hyphenated) and search by slug
                // regardless of entity_type
                let slug: String = target
                    .to_lowercase()
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("-");
                match db::find_note_by_slug(pool, &slug).await {
                    Ok(Some(_)) => {} // target exists
                    Ok(None) => {
                        broken_targets.push(target.to_string());
                    }
                    Err(e) => {
                        eprintln!("[tender]   error checking wikilink '{}': {e}", target);
                    }
                }
            }

            if broken_targets.is_empty() {
                continue;
            }

            if dry_run {
                eprintln!(
                    "[tender]   would fix {} broken wikilink(s) in note '{}' ({})",
                    broken_targets.len(),
                    note.name,
                    note.match_key
                );
            } else {
                match db::remove_broken_wikilinks(pool, &note.id, &broken_targets).await {
                    Ok(()) => {
                        total_fixed += broken_targets.len() as i64;
                        audit_auto_fix(
                            pool,
                            "wikilink",
                            Some(&note.match_key),
                            None,
                            &format!(
                                "Removed {} broken wikilink(s) from note '{}'",
                                broken_targets.len(),
                                note.name
                            ),
                        )
                        .await;
                    }
                    Err(e) => {
                        eprintln!(
                            "[tender]   failed to fix wikilinks in note {}: {e}",
                            note.id
                        );
                    }
                }
            }
        }

        // In apply mode, remove_broken_wikilinks bumps updated_at, shifting
        // the note to the end of the ORDER BY. Keep offset=0 so the next batch
        // surfaces the next un-processed note. In dry_run nothing changes, so
        // advance the offset normally.
        if dry_run {
            offset += batch_size;
        }
    }

    Ok(total_fixed)
}

/// Detect circular references in the edge graph and flag them.
async fn check_circular_refs(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;

    loop {
        let cycles = db::get_circular_refs(pool, batch_size, offset).await?;
        if cycles.is_empty() {
            break;
        }

        if dry_run {
            eprintln!(
                "[tender]   would flag {} circular reference(s) (offset {offset})",
                cycles.len()
            );
            total += cycles.len() as i64;
            offset += batch_size;
            continue;
        }

        for edge in &cycles {
            let related = vec![edge.source_note_id.clone(), edge.target_note_id.clone()];
            match insert_or_update_flag(
                pool,
                "circular",
                "high",
                None,
                Some(&related),
                Some(&format!(
                    "Circular reference detected: edge {} ({}) between notes {} and {}",
                    edge.id, edge.edge_type, edge.source_note_id, edge.target_note_id
                )),
                Some(0.9),
            )
            .await
            {
                Ok(is_new) => {
                    if is_new {
                        total += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[tender]   failed to flag circular ref {}: {e}", edge.id);
                }
            }
        }
        offset += batch_size;
    }

    Ok(total)
}

/// Validate that edge types are consistent with the entity types they connect.
async fn check_type_consistency(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;

    loop {
        let violations = db::get_type_violations(pool, batch_size, offset).await?;
        if violations.is_empty() {
            break;
        }

        if dry_run {
            eprintln!(
                "[tender]   would flag {} type violation(s) (offset {offset})",
                violations.len()
            );
            total += violations.len() as i64;
            offset += batch_size;
            continue;
        }

        for v in &violations {
            let desc = format!(
                "Type violation: edge '{}' connects {} -> {} but expects different entity types",
                v.edge_type, v.source_entity_type, v.target_entity_type
            );
            match insert_or_update_flag(
                pool,
                "type_consistency",
                "medium",
                None,
                None,
                Some(&desc),
                Some(0.7),
            )
            .await
            {
                Ok(is_new) => {
                    if is_new {
                        total += 1;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[tender]   failed to flag type violation for edge {}: {e}",
                        v.edge_id
                    );
                }
            }
        }
        offset += batch_size;
    }

    Ok(total)
}

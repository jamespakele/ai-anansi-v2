---
title: 'Web Tender — DB Maintenance Crawler + Audit'
type: 'feature'
created: '2026-06-28'
baseline_commit: '4b353ec'
status: 'done'
context:
  - 'docs/web-tender-handoff.md'
  - 'docs/anansi-v2-spec.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The Anansi knowledge graph accumulates drift over time — dangling edges, template misalignment, duplicate notes, broken wikilinks, orphaned entities. There is no automated mechanism to detect, flag, or repair these issues.

**Approach:** Build the Web Tender — a background maintenance crawler that periodically scans the DB and performs high-confidence auto-fixes (edge integrity, template alignment, deduplication, index sync, wikilink validity, normalization), flags lower-confidence items to a `tender_queue` table for human review, and provides an on-demand audit skill that renders the queue as a readable markdown report.

## Boundaries & Constraints

**Always:**
- DB is the source of truth — the wiki is regenerated from the DB, not the other way around
- Every auto-fix is logged; anything the tender can't auto-fix with high confidence goes into the queue
- All tender operations are idempotent — running twice produces the same result
- Batch-processed — process N notes per run (configurable, default 100), ordered by `updated_at` ascending
- Rules-driven — behavior changes by editing markdown files, not code
- Follow existing patterns: `tokio::spawn` watcher in `cmd_serve`, `Arc<Config>` + `PgPool`, `pub mod` in `lib.rs`

**Ask First:**
- Whether the tender should run as a tokio task inside the anansi server process or as a separate systemd timer
- Whether to use the existing `%Rules/` directory or create a new `rules/` directory for tender-specific rules

**Never:**
- No HTTP/web crawling — the tender crawls the DB only
- No destructive operations without a queue entry
- No changes to the existing crawl.rs wiki maintenance logic

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Dry run | `--dry-run` flag | Reports what it would do without modifying DB | N/A |
| Apply run | `--apply` flag | Executes auto-fixes, writes flags to queue | Log error per note, continue batch |
| Empty DB | No notes or edges | Reports 0 items processed, exits cleanly | N/A |
| Dangling edge | Edge references deleted note | Edge is removed, logged | N/A |
| Duplicate edge | Two identical (source, target, type) rows | One kept, other removed | N/A |
| Conflicting edges | `works_at` and `competitor_of` between same pair | Flagged to queue with `category: 'conflicting_edges'` | N/A |
| Template drift | Note fields don't match current template | Note re-atomized through sb-atomize pipeline | Log error if re-atomization fails |
| Broken wikilink | `[[missing-entity]]` in note content | Wikilink removed, logged | N/A |
| Exact duplicate | Two notes with same (normalized_name, entity_type) | Merged (union why/content, combine edges, keep oldest, tombstone) | Log if merge conflicts |
| Semantic near-duplicate | Two notes with high embedding similarity | Flagged to queue with similarity score | N/A |
| Orphan note | Note with zero edges | Flagged to queue | N/A |
| Stale/empty note | Note with empty why and content | Flagged to queue | N/A |
| Circular reference | A→B→C→A edge cycle | Flagged to queue with cycle path | N/A |
| Type consistency violation | `reports_to` edge connecting org→org | Flagged to queue | N/A |

> **Scope note:** The I/O matrix above covers the initial implementation. The
> following checks are deferred to a future iteration: index/outline sync,
> normalization rules, template alignment, source integrity, edge inference,
> and semantic deduplication. These are listed in the priority order below but
> marked as deferred.

</frozen-after-approval>

## Code Map

- `migrations/0003_tender_queue.sql` -- Migration to create the `tender_queue` table and indexes
- `src/config.rs` -- Add `TenderConfig` struct with `enabled`, `interval_secs`, `batch_size`, `dry_run` fields
- `src/db.rs` -- Add tender_queue CRUD functions: `insert_tender_flag`, `get_open_flags`, `resolve_flag`, `dismiss_flag`, `get_dangling_edges`, `get_duplicate_edges`, `get_conflicting_edges`, `get_orphan_notes`, `get_stale_notes`, `get_circular_refs`, `get_type_violations`, `merge_duplicate_notes`, `remove_dangling_edge`, `remove_duplicate_edge`, `verify_wikilink_targets`
- `src/tender.rs` -- New module: `run_tender_watcher` loop, check functions for each maintenance category, dry-run/apply logic, batch processing
- `src/lib.rs` -- Add `pub mod tender;`
- `src/main.rs` -- Wire `run_tender_watcher` spawn in `cmd_serve`
- `%Rules/normalization.md` -- Name casing, punctuation, slug rules
- `%Rules/deduplication.md` -- Thresholds for exact vs. semantic match
- `%Rules/edge-inference.md` -- When to auto-add edges from content
- `%Rules/template-mapping.md` -- Which template fields map to which DB columns
- `llm/plugins/anansi.plugin/skills/web-tender/SKILL.md` -- Skill to invoke the tender (dry-run, apply, status)
- `llm/plugins/anansi.plugin/skills/web-tender-audit/SKILL.md` -- Skill to read the queue and produce a markdown report

## Tasks & Acceptance

**Execution:**
- [x] `migrations/0003_tender_queue.sql` -- Create `tender_queue` table with id, category, severity, match_key, related_keys, description, confidence, status, created_at, updated_at, resolved_at, resolved_by columns + indexes on status/category/severity
- [x] `src/config.rs` -- Add `TenderConfig` with `enabled` (default false), `interval_secs` (default 3600), `batch_size` (default 100), `dry_run` (default true); add `#[serde(default)]` and `Default` impl following `WikiConfig` pattern
- [x] `src/db.rs` -- Add tender_queue CRUD: `insert_tender_flag`, `get_open_flags`, `resolve_flag`, `dismiss_flag`; add query functions for each check category (dangling edges, duplicate edges, conflicting edges, orphan notes, stale notes, circular refs, type violations, wikilink targets, exact duplicates)
- [x] `src/db.rs` -- Add mutation functions: `remove_dangling_edge`, `remove_duplicate_edge`, `merge_duplicate_notes`, `remove_broken_wikilinks`
- [x] `src/tender.rs` -- New module with `run_tender_watcher` loop, per-check functions, batch iteration, dry-run vs apply mode, logging
- [x] `src/lib.rs` -- Add `pub mod tender;`
- [x] `src/main.rs` -- Spawn `tender::run_tender_watcher` in `cmd_serve` when `tender.enabled` is true
- [x] `%Rules/normalization.md` -- Initial normalization rules (name casing, punctuation, slug format)
- [x] `%Rules/deduplication.md` -- Initial dedup thresholds (exact match on normalized_name+entity_type, semantic threshold at 0.92 cosine)
- [x] `%Rules/edge-inference.md` -- Initial edge inference rules (when to suggest edges from content mentions)
- [x] `%Rules/template-mapping.md` -- Initial template-to-DB column mapping
- [x] `llm/plugins/anansi.plugin/skills/web-tender/SKILL.md` -- Skill with `--dry-run` and `--apply` modes, status check, log review
- [x] `llm/plugins/anansi.plugin/skills/web-tender-audit/SKILL.md` -- Skill that reads `tender_queue WHERE status = 'open'`, groups by category, sorts by severity then age, renders markdown report with `--resolve` and `--dismiss` flags

**Acceptance Criteria:**
- Given a running anansi instance with `tender.enabled = true`, when the tender interval elapses, then it processes up to `batch_size` notes and performs high-confidence auto-fixes
- Given a dangling edge in the DB, when the tender runs in apply mode, then the edge is removed and logged
- Given a conflicting edge pair, when the tender runs, then a flag is inserted into `tender_queue` with `status = 'open'`
- Given the audit skill is invoked, when the queue has open items, then a markdown report is produced grouped by category and sorted by severity
- Given the audit skill with `--resolve`, when the report is generated, then flagged items are marked as `resolved`

## Design Notes

The tender follows the same watcher pattern as the existing inbox and crawl watchers:

```rust
// In cmd_serve:
if config.tender.enabled {
    let t_config = Arc::clone(&config);
    let t_pool = db.clone();
    tokio::spawn(async move {
        tender::run_tender_watcher(t_config, t_pool).await;
    });
}
```

The `run_tender_watcher` loop:
1. Sleep for `interval_secs`
2. Load rules from `%Rules/`
3. Fetch next batch of notes (oldest `updated_at` first, up to `batch_size`)
4. For each note, run all checks in priority order (high-confidence auto-fix first, low-confidence flag-only last)
5. In dry-run mode, collect what would be done and log it
6. In apply mode, execute auto-fixes and insert queue entries for flags
7. Log summary to `tender_log` table or stdout

Check priority order (highest confidence first):
1. Edge integrity (remove dangling) — High confidence
2. Duplicate edges (remove extras) — High confidence
3. Index/outline sync — High confidence *(deferred)*
4. Wikilink validity — High confidence
5. Normalization rules — High confidence *(deferred)*
6. Template alignment — High confidence *(deferred)*
7. Exact deduplication — High confidence
8. Source integrity — Medium confidence (flag) *(deferred)*
9. Edge inference — Medium confidence (flag) *(deferred)*
10. Conflicting edges — Low confidence (flag)
11. Semantic deduplication — Low confidence (flag) *(deferred)*
12. Orphan detection — Low confidence (flag)
13. Stale/empty notes — Low confidence (flag)
14. Circular references — Low confidence (flag)
15. Type consistency — Medium confidence (flag)

## Verification

**Commands:**
- `cargo build` -- expected: compiles without errors
- `cargo test` -- expected: all tests pass, including new tender tests
- `cargo clippy` -- expected: no new warnings

**Manual checks:**
- Start anansi with `tender.enabled = true` and verify the watcher spawns without error
- Run `tender::run_tender_watcher` in dry-run mode against a test DB with known issues and verify the report
- Run in apply mode and verify auto-fixes are applied and queue entries are created
- Invoke the web-tender-audit skill and verify the markdown report format

## Suggested Review Order

**Entry point — watcher spawn & config**

- Tender watcher spawned alongside existing watchers in cmd_serve
  [`main.rs:586`](../../src/main.rs#L586)

- TenderConfig struct with enabled/interval/batch_size/dry_run
  [`config.rs:218`](../../src/config.rs#L218)

- Module registration in lib.rs
  [`lib.rs:18`](../../src/lib.rs#L18)

**DB schema — migration**

- tender_queue table with indexes for status/category/severity queries
  [`migrations/0003_tender_queue.sql`](../../migrations/0003_tender_queue.sql)

**DB layer — query & mutation functions**

- CRUD: insert, get_open, resolve, dismiss, find_open_flag
  [`db.rs:610`](../../src/db.rs#L610)

- Dangling edge detection + removal
  [`db.rs:705`](../../src/db.rs#L705)

- Duplicate edge detection + removal
  [`db.rs:726`](../../src/db.rs#L726)

- Conflicting edge detection (works_at vs competitor_of, etc.)
  [`db.rs:741`](../../src/db.rs#L741)

- Orphan + stale note detection
  [`db.rs:760`](../../src/db.rs#L760)

- Circular reference detection via recursive CTE
  [`db.rs:783`](../../src/db.rs#L783)

- Type consistency validation (edge type × entity type rules)
  [`db.rs:821`](../../src/db.rs#L821)

- Exact duplicate merge with transaction + contribution re-pointing
  [`db.rs:894`](../../src/db.rs#L894)

- Broken wikilink removal with transaction
  [`db.rs:954`](../../src/db.rs#L954)

**Tender module — watcher loop & check functions**

- Watcher loop with configurable interval, batch processing
  [`tender.rs:49`](../../src/tender.rs#L49)

- TenderReport struct tracking all counters
  [`tender.rs:79`](../../src/tender.rs#L79)

- run_tender_pass orchestrating all 9 checks
  [`tender.rs:90`](../../src/tender.rs#L90)

- Edge integrity check (auto-fix dangling edges)
  [`tender.rs:180`](../../src/tender.rs#L180)

- Duplicate edge check (auto-fix)
  [`tender.rs:203`](../../src/tender.rs#L203)

- Conflicting edges check (flag to queue)
  [`tender.rs:244`](../../src/tender.rs#L244)

- Orphan notes check (flag to queue)
  [`tender.rs:291`](../../src/tender.rs#L291)

- Stale notes check (flag to queue)
  [`tender.rs:325`](../../src/tender.rs#L325)

- Exact duplicate merge (auto-fix with transaction)
  [`tender.rs:365`](../../src/tender.rs#L365)

- Wikilink validity check (auto-fix broken links)
  [`tender.rs:410`](../../src/tender.rs#L410)

- Circular reference check (flag to queue)
  [`tender.rs:480`](../../src/tender.rs#L480)

- Type consistency check (flag to queue)
  [`tender.rs:520`](../../src/tender.rs#L520)

**Rules files — behavior configuration**

- Normalization rules (casing, punctuation, slugs)
  [`%Rules/normalization.md`](../../%Rules/normalization.md)

- Deduplication thresholds (exact + semantic)
  [`%Rules/deduplication.md`](../../%Rules/deduplication.md)

- Edge inference rules (content mention → edge suggestion)
  [`%Rules/edge-inference.md`](../../%Rules/edge-inference.md)

- Template-to-DB column mapping
  [`%Rules/template-mapping.md`](../../%Rules/template-mapping.md)

**Skills — user-facing interface**

- Web tender skill (dry-run/apply/status)
  [`llm/plugins/anansi.plugin/skills/web-tender/SKILL.md`](../../llm/plugins/anansi.plugin/skills/web-tender/SKILL.md)

- Web tender audit skill (queue report with resolve/dismiss)
  [`llm/plugins/anansi.plugin/skills/web-tender-audit/SKILL.md`](../../llm/plugins/anansi.plugin/skills/web-tender-audit/SKILL.md)

---
title: 'Web Tender — Code Review Fixes (Round 1)'
type: 'bugfix'
created: '2026-06-29'
status: 'done'
baseline_commit: 'e55173f'
context:
  - '_bmad-output/implementation-artifacts/spec-web-tender.md'
  - 'docs/web-tender-handoff.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The initial Web Tender implementation (commit `e55173f`) has three critical gaps from a code review: auto-fix checks delete data without queue entries, 7 of 9 check functions load entire tables into memory instead of batching, and the tender does not consume the already-loaded `RuleRegistry` (rules are loaded at startup but never passed to `run_tender_pass`). Additionally, there is no persistent run log and no on-demand invocation path for the skill.

**Approach:** Fix the constraint violations in priority order. First, write a `tender_queue` row for every auto-fix action so no destructive operation goes unlogged. Second, add LIMIT/OFFSET pagination to the 7 unbounded queries. Third, pass the existing `RuleRegistry` into `run_tender_pass` and use its values instead of hardcoded thresholds. Fourth, move inline SQL from `tender.rs` into `db.rs`. Fifth, add a `tender_log` table for persistent run summaries. Fix the ignored `_batch_size` parameter.

## Boundaries & Constraints

**Always:**
- Every auto-fix (edge deletion, duplicate removal, note merge, wikilink fix) MUST write a `tender_queue` row with `severity = 'info'` and `status = 'resolved'` and `resolved_by = 'auto'` — no destructive operation without a trail
- All 9 check functions MUST use batched queries — no `SELECT *` without `LIMIT`
- The tender MUST log a persistent run summary to a `tender_log` table (not just stdout)
- All SQL queries MUST live in `src/db.rs` — never inline in `src/tender.rs`
- The `web-tender` skill MUST be invocable on demand (not just as a background watcher) — add an MCP tool that triggers `run_tender_pass()`
- Keep existing patterns: `tokio::spawn` watcher, `Arc<Config>` + `PgPool`, `pub mod`, `db::get_*` query functions

**Ask First:**
- Whether to add an MCP tool for on-demand tender invocation (if the MCP tool layer is complex) or a simpler CLI subcommand `anansi tender run`

**Never:**
- Do not reimplement the 6 deferred checks from Phase 2
- Do not change the existing watcher spawn pattern
- Do not modify the `tender_queue` schema (it's already correct)
- Do not remove the dry-run vs apply mode gating in any check function

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Edge integrity auto-fix | Dangling edge found in apply mode | Edge deleted, `tender_queue` row inserted with `category='broken_edge'`, `severity='info'`, `status='resolved'`, `resolved_by='auto'` | Log error, continue next edge |
| Large graph (100K+ edges) | `get_dangling_edges()` without LIMIT | Query uses `LIMIT $1 OFFSET $2` with `batch_size`, processes in chunks | Log error per batch, continue |

| No rules files on disk | `%Rules/deduplication.md` missing | Tender logs warning, proceeds with hardcoded defaults (existing behavior preserved) | `eprintln!` warning, continue |
| Run log persistence | Tender pass completes | One row in `tender_log` with notes_processed, edges_removed, duplicates_merged, wikilinks_fixed, flags_inserted, errors, dry_run flag, created_at | Transactional — if insert fails, log to stderr |

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [x] `migrations/0004_tender_log.sql` — Create `tender_log` table with `id UUID PK`, `notes_processed BIGINT`, `edges_removed BIGINT`, `duplicates_merged BIGINT`, `wikilinks_fixed BIGINT`, `flags_inserted BIGINT`, `errors BIGINT`, `dry_run BOOLEAN`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`; add index on `created_at`
- [x] `src/db.rs` — Add `insert_tender_log(pool, report, dry_run)` function that inserts one row into `tender_log`; call it at the end of `run_tender_pass` before returning the report
- [x] `src/db.rs` — Add `LIMIT $1 OFFSET $2` to `get_dangling_edges`, `get_duplicate_edges`, `get_conflicting_edges`, `get_orphan_notes`, `get_stale_notes`, `get_circular_refs`, `get_type_violations`; each function signature gains `limit: i64, offset: i64` parameters
- [x] `src/db.rs` — Add `get_edges_for_group(pool, source_id, target_id, edge_type)` and `get_notes_for_group(pool, match_key, entity_type)` query functions to replace the inline SQL in `tender.rs`
- [x] `src/tender.rs` — Move the two inline SQL queries (`check_duplicate_edges` and `check_exact_duplicates`) into `db.rs` as `get_edges_for_group(source_id, target_id, edge_type)` and `get_notes_for_group(match_key, entity_type)` — following the existing `db::get_*` pattern
- [x] `src/tender.rs` — Add loop-based batching to all 7 check functions that now accept paginated queries; iterate offset by `batch_size` until no rows returned
- [x] `src/tender.rs` — After every auto-fix that mutates the DB (edge deletion, duplicate removal, note merge, wikilink fix), call `insert_tender_flag` with `severity='info'` and `status='resolved'` and `resolved_by='auto'` to create an audit trail
- [x] `src/tender.rs` — Fix `check_exact_duplicates` to actually use the `batch_size` parameter instead of ignoring it with `_batch_size`
- [x] `src/tender.rs` + `src/main.rs` — Pass the existing `RuleRegistry` (already loaded at startup in `cmd_serve`) into `run_tender_watcher` and `run_tender_pass`; use dedup thresholds from the loaded rules instead of hardcoded values; apply normalization rules from the loaded `RuleRegistry` to note name fields before comparison
- [x] `src/mcp.rs` — Add an MCP tool `anansi_tender_run` that accepts `{ dry_run: bool }` and calls `tender::run_tender_pass()` synchronously, returning the `TenderReport` as JSON; add the tool to the `handle_tools_list` JSON array and the `handle_tools_call` match statement; gate on `tender.enabled` config
- [x] `src/config.rs` — No changes needed — `%Rules/` path is already accessible via `config.paths.rules_dir` and `RuleRegistry::load` is already called in `cmd_serve`; the fix is to thread the loaded `RuleRegistry` through to the tender watcher

**Acceptance Criteria:**
- Given a tender pass in apply mode, when an edge is deleted by edge integrity check, then a `tender_queue` row exists with `status='resolved'` and `resolved_by='auto'`
- Given a graph with 500K edges, when `get_dangling_edges` is called with `limit=100, offset=0`, then exactly 100 or fewer rows are returned

- Given a complete tender pass, when it finishes, then one row exists in `tender_log` with accurate counts matching the stdout report
- Given `%Rules/deduplication.md` specifies a dedup threshold of 0.85, when the tender runs, then the exact dedup check uses 0.85 instead of a hardcoded value
- Given the MCP tool `anansi_tender_run` is called with `{ dry_run: true }`, when invoked, then `run_tender_pass` executes and returns a JSON report without mutating the DB
- Given the same MCP tool is called with `{ dry_run: false }`, when invoked, then auto-fixes are applied and queue entries are created

## Design Notes

### Audit trail pattern for auto-fixes

Every mutation that the tender performs must create a queue entry. For auto-fixes:

```rust
// After removing a dangling edge:
db::insert_tender_flag(
    pool,
    "broken_edge",        // category
    "info",               // severity (not a problem, just a record)
    Some(&edge.source_note_id),
    Some(&[edge.target_note_id.clone()]),
    Some("Dangling edge auto-removed"),
    Some(1.0),           // high confidence — we auto-fixed it
).await?;

// Then immediately resolve it so it doesn't clutter the open queue:
db::resolve_auto_flag(pool, &flag.id).await?;
```

This creates a complete audit trail without cluttering the human review queue. The `resolved_by = 'auto'` distinguishes machine actions from human ones.

### Batch loop pattern

```rust
async fn check_edge_integrity(pool: &DbPool, dry_run: bool, batch_size: i64) -> Result<i64> {
    let mut offset = 0i64;
    let mut total = 0i64;
    loop {
        let batch = db::get_dangling_edges(pool, batch_size, offset).await?;
        if batch.is_empty() { break; }
        // ... process batch ...
        total += batch.len() as i64;
        offset += batch_size;
    }
    Ok(total)
}
```

### Rules integration

Rules are already loaded at startup via `RuleRegistry::load(&config.rules_path(root))` in `cmd_serve` (src/main.rs L493). The fix is to thread the loaded `RuleRegistry` into the tender watcher, not to re-read files:

```rust
// In cmd_serve, after loading rules:
let rules = RuleRegistry::load(&config.rules_path(root)).unwrap_or_default();
// Pass rules into the tender watcher spawn:
tender::run_tender_watcher(t_config, t_pool, Arc::new(rules))
```

Then `run_tender_pass` reads dedup thresholds, normalization rules, and edge type mappings from the `RuleRegistry` instead of hardcoded values.

## Verification

**Commands:**
- `cargo build` -- expected: compiles without errors
- `cargo test` -- expected: all tests pass
- `cargo clippy` -- expected: no new warnings

**Manual checks:**
- Start anansi, call `anansi_tender_run { dry_run: true }` via MCP, verify JSON report is returned
- Check `tender_queue` after apply mode — every auto-fix must have a corresponding row
- Check `tender_log` table — one row per completed pass with accurate counts
- Verify conflicting edges check no longer flags identical edge types between same pair

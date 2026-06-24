---
title: 'Anansi v2 — Build 13: anansi-crawl Core (wiki↔PG reconcile + access tracking)'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: 'a673343'
context:
  - _bmad-output/implementation-artifacts/spec-build-12-llm-wiki-notestore.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Build-12 dual-writes the LLM-wiki on capture, but nothing keeps it healthy over time: notes captured before the wiki was enabled have no files, a failed/skipped materialize leaves stale or missing files, deletes outside the wired paths orphan files, and `index.md` can drift. There is also no record of when a note was last read — required to order maintenance and (later) evict cold notes.

**Approach:** Add `anansi-crawl`: the mechanical maintenance pass of the Karpathy "lint" loop. (1) A new `last_accessed_at` column on `notes`, bumped when a note is read via the MCP read tools (`anansi_get`/`search`/`filter`/`search_semantic`) — user reads only, not internal reads. (2) A `WikiStore::crawl` that, ordered by last-access (coldest first), re-projects every live note from canonical DB state (reusing `materialize` to repair missing/stale files), garbage-collects orphan wiki files no live note backs, rebuilds `index.md`, and appends a crawl summary to `log.md`. (3) Two triggers: a background Tokio task on a config interval (mirrors the queue/inbox watchers) and an on-demand `anansi_wiki_crawl` MCP tool. This is Goal B part 1 (mechanical). The LLM semantic lint (Build-14) and tipping-point eviction (Build-15) are deferred.

## Boundaries & Constraints

**Always:**
- Postgres is the source of truth; the crawl only ever reads notes/edges from PG and writes the wiki projection — it never mutates note content in PG. (It MAY write `last_accessed_at`, and MAY write `conflicts` only in Build-14, not here.)
- `last_accessed_at` is bumped ONLY by the user-facing MCP read tools, never by internal `get_note`/`edges_for_note` calls (those run inside materialize and would corrupt the cold/hot signal).
- Access-time updates and the whole crawl are non-fatal: failures are logged (`[crawl]`/`[wiki]`) and never fail the triggering read or block the event loop.
- The crawl reuses `WikiStore::materialize` for reconciliation (no second serializer) and only removes wiki files it can prove no live note maps to.
- Orphan GC only deletes `*.md` files under the wiki root that are neither `index.md`/`log.md` nor a current live-note filename; it never touches files outside the wiki root.
- Backward compatible: crawl is gated by `[wiki] crawl_enabled` (default false) for the background task. The migration is additive (nullable column) and backfills all existing rows with the migration-run timestamp — a uniform "everything present at start" epoch, so pre-existing notes all sort as equally-coldest until real reads differentiate them. When `wiki.enabled = false`, the crawl and the MCP tool are no-ops/disabled.

**Ask First:**
- Default crawl interval (proposed `3600s`).
- Whether the `anansi_wiki_crawl` MCP tool should be allowed when `wiki.enabled = false` (proposed: return a disabled message, do nothing).

**Never:**
- Do not implement LLM semantic lint — contradictions, stale-claim detection, under-linked analysis (Build-14).
- Do not implement size caps or eviction (Build-15); the crawl re-projects ALL live notes, it does not bound the wiki.
- Do not add `last_accessed_at` to the `NoteRecord` struct (avoids churn across every literal); query it directly where needed.
- Do not bump access time on writes or internal reads.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| MCP read of a note | `anansi_get`/`search`/`filter`/`semantic` returns notes | `last_accessed_at = now` for the returned note id(s) | bump failure logged, read still returns |
| Crawl run, wiki enabled | background tick or `anansi_wiki_crawl` | every live note re-projected, orphans removed, index rebuilt, `log.md` gets `## [date] crawl \| N notes, M orphans` | per-note errors logged, crawl continues |
| Missing wiki file for old note | note predates wiki / prior failure | file recreated from canonical state | N/A |
| Orphan file (note deleted) | `<slug>.<type>.md` with no live note | file removed | missing/locked file logged, skipped |
| Archived note | `entity_type` `archive-%` | excluded from projection + treated as non-backing (its stale file is GC'd) | N/A |
| Crawl, wiki disabled | `wiki.enabled = false` | no-op; MCP tool returns `disabled` | N/A |
| `crawl_enabled = false` (default) | server start | background crawl task not spawned; MCP tool still usable | log "crawl watcher disabled" |

</frozen-after-approval>

## Code Map

- `migrations/0002_last_accessed_at.sql` -- NEW: `ALTER TABLE notes ADD COLUMN last_accessed_at TEXT;` then `UPDATE notes SET last_accessed_at = <rfc3339 now> WHERE last_accessed_at IS NULL;` to backfill the epoch. `sqlx::migrate!` runs it on connect.
- `src/db.rs` -- add `touch_access(pool, ids: &[String])` (UPDATE last_accessed_at = now WHERE id = ANY($1)); add `live_note_ids_by_access(pool) -> Vec<String>` (SELECT id FROM notes WHERE entity_type NOT LIKE 'archive-%' ORDER BY last_accessed_at ASC NULLS FIRST). Do NOT touch `NoteRecord`/`row_to_note`.
- `src/wiki.rs` -- add `CrawlReport { notes_projected, orphans_removed, errors }` and `pub async fn crawl(&self, pool) -> Result<CrawlReport>`: no-op when disabled; fetch `live_note_ids_by_access`; `materialize(pool, &ids, "crawl")` to reconcile + rebuild index; compute expected filename set from live notes; scan root for `*.md` (excluding `index.md`/`log.md`) and `remove_file` any not in the set; append `## [date] crawl | N notes, M orphans removed` to `log.md`. Add `expected_filename(entity_type, name)` helper (shares `slug_name`).
- `src/crawl.rs` -- NEW: `run_crawl_watcher(config: Arc<Config>, pool: DbPool)` — poll loop every `wiki.crawl_interval_secs`, build `WikiStore::from_config`, call `crawl`, log the report. Mirrors `queue.rs`.
- `src/config.rs` -- extend `WikiConfig` with `crawl_enabled: bool` (default false) and `crawl_interval_secs: u64` (default 3600).
- `src/lib.rs` -- add `pub mod crawl;`.
- `src/main.rs` -- in `cmd_serve`, after the inbox watcher: if `wiki.enabled && wiki.crawl_enabled`, `tokio::spawn(crawl::run_crawl_watcher(...))`; else log disabled.
- `src/mcp.rs` -- bump access time in `tool_get`/`tool_search`/`tool_filter`/`tool_search_semantic` via `db::touch_access` on the returned ids (non-fatal); add `anansi_wiki_crawl` dispatch + `tool_wiki_crawl` handler (read-skill gated, respects `read_only`/`wiki.enabled`) returning the `CrawlReport`.
- `anansi.toml.example` -- document `crawl_enabled` / `crawl_interval_secs` under `[wiki]`.

## Tasks & Acceptance

**Execution:**
- [x] `migrations/0002_last_accessed_at.sql` -- additive nullable column + backfill existing rows with the migration-run rfc3339 timestamp.
- [x] `src/config.rs` -- add `crawl_enabled` (default false) + `crawl_interval_secs` (default 3600) to `WikiConfig` and its `Default`.
- [x] `src/db.rs` -- add `touch_access` and `live_note_ids_by_access`.
- [x] `src/wiki.rs` -- add `CrawlReport`, `crawl`, and `expected_filename`; reconcile via `materialize`, GC orphans, append crawl log line. Unit-test orphan-detection (filename set membership) and the disabled no-op.
- [x] `src/crawl.rs` -- NEW background watcher; add `pub mod crawl;` to `src/lib.rs`.
- [x] `src/main.rs` -- spawn crawl watcher when `wiki.enabled && wiki.crawl_enabled`.
- [x] `src/mcp.rs` -- access-time bumps in the 4 read tools; `anansi_wiki_crawl` tool + dispatch.
- [x] `anansi.toml.example` -- document the new `[wiki]` crawl keys.

**Acceptance Criteria:**
- Given `cargo check` and `cargo test --no-run`, when run after all changes, then both finish with zero errors.
- Given a note that has no wiki file and `wiki.enabled = true`, when the crawl runs, then its `<slug>.<entity_type>.md` is created and `index.md` lists it.
- Given a `*.md` file under the wiki root that no live note backs, when the crawl runs, then the file is removed and the crawl `log.md` line reports it.
- Given `anansi_get` is called for a note, when it returns, then that note's `last_accessed_at` is set to ~now (and internal materialize reads do NOT change it).
- Given `wiki.enabled = false`, when `anansi_wiki_crawl` is invoked, then it returns a `disabled` status and writes nothing.
- Given `crawl_enabled = false` (default), when the server starts, then no crawl background task is spawned and the log says so.

## Spec Change Log

- **v1.1 (review patches, 2026-06-24):** Three-reviewer adversarial pass; no intent_gap/bad_spec, no loopback. Patched: (1) HIGH — `last_accessed_at` mis-ordered because the migration backfills fixed 6-digit microseconds while `now_rfc3339()` is chrono AutoSi (variable precision); `touch_access` now uses new `db::now_rfc3339_micros()` (fixed µs, `+00:00`) matching the migration so lexical sort == chronological. (2) MEDIUM — crawl GC could delete a live note's file written by a concurrent capture between the DB snapshot and the dir sweep; added an mtime guard (spare files modified at/after `crawl_start`) and skip GC entirely if projection errored. (3) MEDIUM — O(N) write churn every crawl; `atomic_write` now skips identical rewrites (covers note files + `index.md`). Rejected (reviewers confirmed safe): slug-lockstep (note_path delegates to expected_filename, unit-tested), archive-filter consistency (both queries exclude `archive-%`), `.tmp` skipped by `.md` filter, access scoped to user reads. KEEP: projection-writer reuse for reconcile, MCP-layer access bumping, coldest-first ordering, mechanical-only scope.

## Design Notes

**Why bump access time at the MCP layer, not in `db::get_note`.** `materialize` calls `get_note`/`edges_for_note` heavily for internal projection; bumping there would mark every note "hot" on every crawl and destroy the cold/hot signal that Build-15 eviction depends on. Only the four user-facing read tools bump.

**Crawl = reconcile + GC, reusing materialize:**
```text
ids = live_note_ids_by_access(pool)         // coldest first
materialize(pool, ids, "crawl")             // recreate missing/stale files + rebuild index.md
expected = { expected_filename(t,n) for each live note }  // ∪ index.md, log.md
for f in root/*.md: if f ∉ expected: remove_file(f); orphans++
append log: "## [date] crawl | {ids.len()} notes, {orphans} orphans removed"
```
Reconciliation is idempotent — `materialize` overwrites with canonical content (atomic write), so re-running is safe and self-healing (covers the Build-12 `already_ingested` gap noted in deferred-work).

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.
- `cargo test --lib wiki` -- expected: existing + new orphan/disabled unit tests pass.

**Manual checks:**
- With `[wiki] enabled=true crawl_enabled=true`, start the server, touch a stray `bogus.note.md` into `/data/llm-wiki/`, wait one interval (or call `anansi_wiki_crawl`), confirm `bogus.note.md` is gone and `log.md` shows a crawl line; `anansi_get` a note, then `SELECT last_accessed_at FROM notes WHERE id=...` is non-null.

## Suggested Review Order

**The crawl (start here)**

- Entry point — reconcile + orphan-GC + journal; the whole maintenance pass.
  [`wiki.rs:132`](../../src/wiki.rs#L132)
- Shared projection extracted from materialize — reused by crawl, no second serializer.
  [`wiki.rs:71`](../../src/wiki.rs#L71)
- GC safety: mtime guard spares concurrently-written files (review-driven).
  [`wiki.rs:142`](../../src/wiki.rs#L142)
- Orphan-GC lockstep: `expected_filename` must equal `note_path` (unit-tested).
  [`wiki.rs:386`](../../src/wiki.rs#L386)

**Access tracking**

- Fixed-µs timestamp so lexical sort == chronological (review-driven fix).
  [`db.rs:39`](../../src/db.rs#L39)
- `touch_access` + coldest-first ordering query.
  [`db.rs:235`](../../src/db.rs#L235)
- Bumped only by user-facing reads, never internal.
  [`mcp.rs:549`](../../src/mcp.rs#L549)

**Triggers**

- On-demand MCP tool (`anansi_wiki_crawl`), disabled-safe.
  [`mcp.rs:558`](../../src/mcp.rs#L558)
- Background watcher mirrors queue/inbox.
  [`crawl.rs:18`](../../src/crawl.rs#L18)
- Spawned only when wiki + crawl both enabled.
  [`main.rs:423`](../../src/main.rs#L423)

**Peripherals**

- Idempotent migration: nullable column + epoch backfill.
  [`0002_last_accessed_at.sql`](../../migrations/0002_last_accessed_at.sql)
- Config gate (default off).
  [`config.rs:206`](../../src/config.rs#L206)
- Skip-unchanged write avoids crawl churn.
  [`wiki.rs:445`](../../src/wiki.rs#L445)

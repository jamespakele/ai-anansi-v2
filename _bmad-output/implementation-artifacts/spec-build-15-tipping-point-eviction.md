---
title: 'Anansi v2 — Build 15: Tipping-Point Eviction (Goal C)'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '096bba7'
context:
  - _bmad-output/implementation-artifacts/spec-build-13-anansi-crawl-core.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The LLM-wiki currently grows unbounded — the crawl projects every live note to a file. Past some folder size the file-based wiki stops being worth it; it should be capped to the hottest (most-recently-accessed) notes that fit, with the rest living only in Postgres. Nothing today enforces a tipping point.

**Approach:** Make the crawl **size-bounded**. Order live notes hottest-first by `last_accessed_at`, project them until a configurable cap is hit — **either** total bytes (`max_mb`) **or** file count (`max_notes`), whichever comes first — and treat every colder note beyond the cap as **evicted**: its file is removed (it already falls out of the crawl's orphan-GC because only resident notes go into the GC's "expected" set), and `index.md` lists only resident notes. Eviction is **lossless**: reads still resolve via MCP→Postgres (the wiki was never read-through), so the wiki is simply a bounded, browsable LRU cache over the backend. Both caps default to `0` (unlimited) → the crawl behaves exactly as Build-13/14. This is Goal C; it completes the LLM-wiki epic.

## Boundaries & Constraints

**Always:**
- Eviction only ever deletes wiki **files**; the note's Postgres row (content, edges, conflicts) is never touched. Eviction is lossless — an evicted note is fully readable via `anansi_get`/`search`.
- Caps default to `0 = unlimited`. With both unlimited, the crawl is byte-for-byte Build-13/14 behavior (resident set = all live notes). Backward compatible.
- The **resident set** = live notes ordered by `last_accessed_at` DESC (hottest first), accumulated until either cap is reached: stop when `max_notes > 0 && resident_count >= max_notes`, OR when `max_mb > 0 && resident_bytes >= max_mb*1MB`. Whichever triggers first bounds the set.
- Only resident notes are projected and listed in `index.md`. The crawl's orphan-GC `expected` set = resident filenames (+ reserved `index.md`/`log.md`/`lint.md`), so non-resident (evicted) note files are removed by the same GC pass that removes true orphans — preserving the Build-13 mtime-guard and skip-on-projection-error safety.
- The capture/materialize path (`anansi_capture`, ingest) is NOT cap-aware — it always writes the just-touched note (which is by definition hot); the crawl re-bounds the set on its next pass. Only the crawl enforces caps.
- Non-fatal/best-effort throughout, consistent with Build-13.

**Ask First:**
- Whether a just-captured note that would exceed the cap should be written anyway (proposed: yes — it's hot; the crawl evicts it later if it goes cold).
- Sensible default once enabled (proposed: keep `0`/unlimited as the shipped default; the operator sets a real cap per system).

**Never:**
- Do not make the wiki read-through or add cache-miss "fault-in" logic — reads already hit Postgres via MCP; eviction needs no fallthrough code.
- Do not evict from the capture path or block writes on cap checks.
- Do not delete or alter Postgres data during eviction.
- Do not let the resident-index/all-index divergence on delete/archive (`refresh_index`) regress correctness — document it as a transient the next crawl heals (do not re-architect `refresh_index` here).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Crawl, caps unlimited (default) | `max_mb=0, max_notes=0` | all live notes projected; identical to Build-13/14 | per-note errors logged, continue |
| Crawl, count cap hit | `max_notes=N` < live count | hottest N projected; colder notes' files removed; index lists N | N/A |
| Crawl, size cap hit | resident bytes ≥ `max_mb*1MB` | projection stops at the cap; colder files removed | N/A |
| Read of an evicted note | `anansi_get` by id/match_key | full note returned from Postgres (no file needed) | N/A |
| Evicted note re-read then crawl | access bumps `last_accessed_at`, next crawl | note re-enters resident set, file recreated | N/A |
| Capture of a cold-ranked note | `anansi_capture` | file written immediately (note is now hot) | N/A |
| Both caps set | `max_mb` and `max_notes` | whichever bound is reached first stops projection | N/A |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- add `max_mb: u64` (default 0) and `max_notes: u64` (default 0) to `WikiConfig` + `Default`.
- `src/db.rs` -- add `live_note_ids_by_access_desc(pool) -> Vec<String>` (live only, `ORDER BY last_accessed_at DESC NULLS LAST`) — hottest first for resident-set selection. (Keep the existing ASC fn for nothing-changes; crawl switches to DESC.)
- `src/wiki.rs` -- add `max_bytes: u64`, `max_notes: u64` to `WikiStore` (from config: `max_mb * 1_048_576`, `max_notes`); add `evicted: usize` to `CrawlReport`. Rewrite `crawl` to: fetch hottest-first ids; walk them, projecting each (write file, measure its byte size, collect resident `NoteRecord`s + filenames) until a cap is reached, counting the rest as `evicted`; rebuild `index.md` from the **resident** notes only (new `rebuild_index_from(&[summary])` or pass resident records); GC with `expected` = resident filenames + reserved (so evicted + orphan files are removed, keeping the mtime-guard + skip-on-error); journal `## [date] crawl | R resident, E evicted, F removed`. Unlimited caps ⇒ resident = all live ⇒ Build-13 behavior. The capture-path `materialize`/`project` stay unchanged (not cap-aware).
- `src/mcp.rs` -- `tool_wiki_crawl` already returns the report; include `evicted` in its JSON.
- `anansi.toml.example` -- document `max_mb` / `max_notes` (0 = unlimited).

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- add `max_mb` (default 0) + `max_notes` (default 0) to `WikiConfig` + `Default`.
- [x] `src/db.rs` -- add `live_note_ids_by_access_desc`.
- [x] `src/wiki.rs` -- add caps to `WikiStore` + `from_config`; add `evicted` to `CrawlReport`; rewrite `crawl` to project the hottest resident set under the caps, build the resident-only index, and GC non-resident files; keep mtime-guard + skip-on-projection-error. Unit-test the resident-set selection (count cap, size cap, unlimited) as a pure helper.
- [x] `src/mcp.rs` -- surface `evicted` in the `tool_wiki_crawl` response.
- [x] `anansi.toml.example` -- document the cap keys.

**Acceptance Criteria:**
- Given `cargo check` / `cargo test --no-run`, when run, then both finish with zero errors; `cargo test wiki` passes new + existing tests.
- Given `max_mb=0, max_notes=0` (default), when a crawl runs, then every live note is projected and GC removes only true orphans — identical to Build-13.
- Given `max_notes=N` and more than N live notes, when a crawl runs, then exactly the N hottest (by `last_accessed_at`) have files, colder notes' files are removed, and `index.md` lists N notes.
- Given a note whose file was evicted, when `anansi_get` is called for it, then the full note is returned from Postgres.
- Given an evicted note is read (bumping `last_accessed_at`) and a crawl runs, then its file is recreated (it re-entered the resident set).
- Given `max_mb` is small, when a crawl runs, then resident bytes stay at or below the cap boundary (projection stops once reached).

## Spec Change Log

- **v1.1 (review patches, 2026-06-24):** Three-reviewer adversarial pass; no intent_gap, no loopback (patches only). Fixed: (1) HIGH — newly-captured notes (`last_accessed_at = NULL`) sorted COLDEST under `NULLS LAST` and were evicted before ever being read; ordering changed to `COALESCE(last_accessed_at, updated_at) DESC, id DESC` (fresh notes are hot; `id` tiebreaker makes backfill-epoch ties deterministic — also fixes the F3 nondeterminism). (2) HIGH — with caps active the capture/refresh paths rebuilt the *all-live* index, listing evicted notes as broken wikilinks (steady-state, not transient); now when `caps_active()` the crawl is the sole index author (`project`/`refresh_index` skip the rebuild). (3) HIGH — `project_failed` skipped GC but still rewrote the resident index, leaving it under-listing vs files; index rebuild moved inside the `!project_failed` gate. (4) MED — a transient `write_note` failure let GC delete the note's good prior file; now sets `project_failed`. (5) MED — GC `read_dir` `?` aborted the crawl after the index was rewritten; now non-fatal (skip GC, still journal). Plus watcher log + soft-cap doc wording. Rejected: per-note query "regression" (Build-13 `project` did the same), `resident_cutoff` test (correctly tests the intended soft cap). Deferred (deferred-work.md): index staleness for a note deleted between crawls when capped; soft size cap (overshoot ≤ one note); 3 pre-existing `prompt::tests` failures unrelated to this work. KEEP: eviction = GC, lossless reads via MCP→PG, capture path not cap-aware, mtime-guard + skip-on-error.

## Design Notes

**Resident-set selection (pure, testable).** Factor the cap walk into a helper:
```rust
// returns (resident_indices_count, stop_reason) given ordered (size_estimate) items
fn resident_cutoff(sizes: &[u64], max_bytes: u64, max_notes: u64) -> usize {
    let mut bytes = 0; let mut n = 0;
    for &sz in sizes {
        if (max_notes > 0 && n >= max_notes) || (max_bytes > 0 && bytes >= max_bytes) { break; }
        bytes += sz; n += 1;
    }
    n
}
```
0 caps mean "unlimited" (the guards are skipped), so resident = all. In `crawl`, size is the actual written file length (`fs::metadata(path).len()`), measured as each resident note is projected; once the next note would cross a cap, the remainder are evicted.

**Why eviction = GC.** The crawl already deletes any `*.md` not in `expected`. Build-15 simply narrows `expected` from "all live notes" to "resident notes," so cold-note files are swept by the existing, already-safe GC (mtime guard spares concurrently-written files; reserved files protected). No separate deletion path.

**Index divergence (documented transient).** `refresh_index` (delete/archive paths) rebuilds from all live notes, so right after a delete/archive the index may briefly list non-resident notes; the next crawl re-bounds it. Acceptable; a cap-aware `refresh_index` is deferred.

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.
- `cargo test wiki` -- expected: resident-cutoff + existing wiki tests pass.

**Manual checks:**
- Set `[wiki] enabled=true crawl_enabled=true max_notes=3`, ensure >3 notes exist with varied `last_accessed_at` (read a few via `anansi_get`), run `anansi_wiki_crawl`, confirm only the 3 hottest have files in `/data/llm-wiki/`, `index.md` lists 3, and `anansi_get` on an evicted note still returns it.

## Suggested Review Order

**The bounded crawl (start here)**

- Entry point — the cap-aware crawl: resident projection, resident index, GC.
  [`wiki.rs:153`](../../src/wiki.rs#L153)
- The cap test (`over`) — count + soft byte cap, hottest-first.
  [`wiki.rs:180`](../../src/wiki.rs#L180)
- Pure cutoff helper (unit-tested at `wiki.rs:678`).
  [`wiki.rs:310`](../../src/wiki.rs#L310)
- Warmth ordering — `COALESCE(last_accessed, updated)` so fresh notes are hot (review-driven).
  [`db.rs:303`](../../src/db.rs#L303)

**Consistency invariants (review-driven)**

- `caps_active` — when capped, the crawl is the sole index author.
  [`wiki.rs:144`](../../src/wiki.rs#L144)
- Index + GC gated together behind `!project_failed` (no index/file skew).
  [`wiki.rs:383`](../../src/wiki.rs#L383)
- A failed `write_note` sets `project_failed` so GC won't delete the good file.
  [`wiki.rs:224`](../../src/wiki.rs#L224)

**Peripherals**

- Caps on config (default 0 = unlimited, soft `max_mb`).
  [`config.rs:221`](../../src/config.rs#L221)

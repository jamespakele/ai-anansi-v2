---
title: 'Anansi v2 — Build 14: LLM Semantic Lint (anansi-crawl pt2)'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '2c6642f'
context:
  - _bmad-output/implementation-artifacts/spec-build-13-anansi-crawl-core.md
  - _bmad-output/implementation-artifacts/spec-build-11-inbox-watcher.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Build-13's crawl keeps the wiki structurally healthy but does no *semantic* maintenance — the analytical half of the Karpathy "lint" loop. Nothing surfaces contradictions between notes, claims gone stale, notes that should be cross-linked but aren't, or concepts the wiki references but never defines. The scaffolded `has_conflicts`/`conflicts_updated_at` columns have never been written.

**Approach:** Add an LLM semantic-lint phase to the crawl. Incrementally (notes changed since a stored watermark, capped per pass so the first-run backlog drains over time), for each target note build a context of the note + its edge-neighbors and ask the configured `LlmClient` for findings across four dimensions — **contradictions, stale claims, under-linking, data gaps** — returned as JSON. Findings surface two ways: contradiction-flagged notes get `has_conflicts = 1` (+ `conflicts_updated_at`) in Postgres, and ALL findings are written to a human-browsable `lint.md` in the wiki. Gated by `[wiki] lint_enabled` (default false) and the presence of an LLM backend. This is Goal B part 2; tipping-point eviction (Build-15) remains deferred.

## Boundaries & Constraints

**Always:**
- Postgres stays the source of truth. The lint MAY set `has_conflicts`/`conflicts_updated_at` on notes (its purpose); it MUST NOT alter note content, names, types, or edges.
- The lint is **incremental**: process only live notes with `updated_at` greater than the stored watermark, ordered oldest-first, at most `lint_batch_max` per pass; advance the watermark to the newest note analyzed. First run (no watermark) drains the backlog in capped batches across successive passes.
- Entirely non-fatal and best-effort: any LLM/parse/DB error for a note is logged (`[lint]`) and skipped; lint failures never fail the mechanical crawl or the server.
- The lint runs only when `wiki.enabled && wiki.lint_enabled` AND an LLM backend is configured/build-able (same `llm::build_client` path the inbox watcher uses). Otherwise it is skipped with a log line.
- `lint.md` and `.lint-state` are reserved wiki files: the crawl's orphan-GC MUST NOT delete them (add `lint.md` to the reserved set; `.lint-state` is already safe as a non-`.md` file).
- LLM output is parsed defensively (strip markdown fences like `pipeline.rs`); malformed JSON for a note is skipped, not fatal.

**Ask First:**
- Default `lint_batch_max` (proposed `25` notes/pass) and whether the lint runs as a phase of the crawl vs only the dedicated MCP tool (proposed: both — a crawl phase AND `anansi_wiki_lint`).
- The per-note prompt wording (the four-dimension JSON contract) — propose in Design Notes; flag if you want changes.

**Never:**
- Do not implement eviction / size caps (Build-15).
- Do not block the event loop or run unbounded LLM calls in one pass (the `lint_batch_max` cap is mandatory).
- Do not delete or rewrite note content based on findings — lint only *flags and reports*, humans decide.
- Do not clear `has_conflicts` here in a way that loses prior conflict state without re-analysis (only set/refresh for notes actually analyzed this pass).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Lint pass, notes changed | `lint_enabled`, LLM configured | up to `lint_batch_max` changed notes analyzed; `lint.md` written; contradiction notes get `has_conflicts=1`; watermark advanced | per-note LLM/parse error logged + skipped |
| First run, large backlog | no `.lint-state` | oldest `lint_batch_max` analyzed this pass; remainder next passes | N/A |
| No changes since watermark | watermark current | no LLM calls; `lint.md` left as-is; log "nothing to lint" | N/A |
| LLM not configured | `lint_enabled` but no backend | lint skipped entirely | log "[lint] no LLM backend — skipping" |
| Malformed LLM JSON for a note | bad model output | that note skipped; others proceed | logged, parse error swallowed |
| `lint_enabled = false` (default) | — | no lint phase; crawl is mechanical-only | N/A |
| Crawl orphan-GC vs lint.md | crawl runs | `lint.md` preserved (reserved) | N/A |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- add `lint_enabled: bool` (default false) and `lint_batch_max: u64` (default 25) to `WikiConfig` + `Default`.
- `src/db.rs` -- add `set_conflict(pool, note_id, has: bool)` (UPDATE has_conflicts + conflicts_updated_at); add `notes_changed_since(pool, watermark: Option<&str>, limit: i64) -> Vec<NoteRecord>` (live only, `updated_at > $1` or all when None, ORDER BY updated_at ASC LIMIT). Reuse `edges_for_note`/`get_note` for neighbor context.
- `src/lint.rs` -- NEW. `LintFinding { kind, note, detail, related }`, `LintReport { analyzed, findings, new_watermark }`; `pub async fn run_lint(pool, wiki: &WikiStore, llm: &dyn LlmClient, batch_max) -> Result<LintReport>`: read watermark from `wiki.root/.lint-state`; fetch changed notes; per note build neighbor context + prompt, `llm.infer` (JSON), parse defensively into findings across the 4 kinds; `db::set_conflict(true)` for notes with contradictions; aggregate; write `wiki.root/lint.md`; write new watermark to `.lint-state`. Embed the prompt via `include_str!("../prompts/lint.txt")`.
- `prompts/lint.txt` -- NEW: the four-dimension lint prompt with `{{NOTE}}` / `{{NEIGHBORS}}` placeholders, instructing strict JSON output.
- `src/wiki.rs` -- add `"lint.md"` to the crawl orphan-GC reserved set; expose a small helper or `pub fn lint_path(&self)`/`state_path(&self)` if useful; ensure `lint.md`/`.lint-state` are never GC'd.
- `src/crawl.rs` -- after the mechanical `crawl`, if `wiki.lint_enabled`: build an LLM client via `llm::build_client(&config.llm)`; if Ok, `lint::run_lint(...)` and log the report; else log skip.
- `src/mcp.rs` -- add `anansi_wiki_lint` dispatch + `tool_wiki_lint` handler (build LLM client from `ctx.config`, run `run_lint`, return finding counts; `disabled` when `!wiki.enabled || !lint_enabled`); register in `tools/list`.
- `src/lib.rs` -- add `pub mod lint;`.
- `anansi.toml.example` -- document `lint_enabled` / `lint_batch_max`.

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- add `lint_enabled` (default false) + `lint_batch_max` (default 25).
- [x] `src/db.rs` -- add `set_conflict` and `notes_changed_since`.
- [x] `prompts/lint.txt` -- four-dimension JSON lint prompt with `{{NOTE}}`/`{{NEIGHBORS}}`.
- [x] `src/lint.rs` -- watermark read/write, per-note neighbor context, LLM call + defensive JSON parse, `has_conflicts` setting, `lint.md` rendering, `LintReport`. Unit-test the JSON parser (well-formed, fenced, malformed→skip) and the `lint.md` renderer.
- [x] `src/wiki.rs` -- reserve `lint.md` in crawl GC; helper paths if needed.
- [x] `src/crawl.rs` -- run lint phase after mechanical crawl when enabled + LLM available.
- [x] `src/mcp.rs` -- `anansi_wiki_lint` tool + dispatch + tools/list entry.
- [x] `src/lib.rs` -- `pub mod lint;`.
- [x] `anansi.toml.example` -- document the lint keys.

**Acceptance Criteria:**
- Given `cargo check` / `cargo test --no-run`, when run, then both finish with zero errors; `cargo test lint` passes the parser/renderer unit tests.
- Given `lint_enabled` with a configured LLM and ≥1 changed note, when a lint pass runs, then `lint.md` is written, at most `lint_batch_max` notes are analyzed, and the `.lint-state` watermark advances.
- Given the LLM flags a contradiction on a note, when the pass completes, then that note's `has_conflicts = 1` and `conflicts_updated_at` is set, visible via `anansi_get`.
- Given a crawl runs after a lint, when orphan-GC executes, then `lint.md` and `.lint-state` are NOT deleted.
- Given `lint_enabled = false` (default) or no LLM backend, when the crawl runs, then no LLM calls occur and the crawl behaves exactly as Build-13.
- Given malformed LLM JSON for a note, when parsed, then that note is skipped and the pass continues.

## Spec Change Log

- **v1.1 (review patches, 2026-06-24):** Three-reviewer adversarial pass; no intent_gap/bad_spec, no loopback. Patched: (1) HIGH — `updated_at > watermark` + LIMIT permanently skipped notes sharing a timestamp at the batch boundary (common after bulk ingest); switched to **keyset pagination on `(updated_at, id)`** with the cursor stored as `updated_at\tid` in `.lint-state`. (2) HIGH — `lint.md` was overwritten each pass, so an incremental backlog left only the last batch visible; now **appends** a per-run section (header written once), like `log.md`. (3) MEDIUM — crawl-phase lint and the `anansi_wiki_lint` tool could run concurrently (double cost, watermark race); added a process-global **single-flight `try_lock`** (skip if already running). (4) MEDIUM — brace-slice parser broke on trailing prose/multiple objects; replaced with a **balanced-brace extractor** respecting string literals. (5) LOW — errored/unparseable notes were uncounted; added `errors`/`skipped` to `LintReport` and the report header (no longer silent); `set_conflict(false)` now sets `conflicts_updated_at = NULL` instead of re-stamping; `.lint-state` write failure now logs loudly. Deferred (see deferred-work.md): transient-error notes skip until re-edited (no retry/dead-letter); contradiction flags only the analyzed note, not its partner; a note isn't re-linted when a *neighbor* changes; no global LLM token/daily budget beyond `batch_max × interval`. KEEP: incremental watermark, single-`infer`-per-note, four-dimension JSON contract, has_conflicts as the contradiction signal.

## Design Notes

**Lint prompt contract (per note).** Input: the target note (`name`, `entity_type`, `lede`, `why`, `content`) and its neighbors (`name`, `entity_type`, `lede`, `edge_type`). Output: strict JSON
```json
{
  "contradictions": [{"with": "<neighbor name>", "detail": "..."}],
  "stale":          [{"detail": "..."}],
  "under_linked":   [{"suggest": "<note name or concept>", "detail": "..."}],
  "gaps":           [{"concept": "...", "detail": "..."}]
}
```
Parsed with the existing fence-stripping approach; any field may be absent/empty. Only `contradictions` (non-empty) sets `has_conflicts`.

**Watermark.** `wiki.root/.lint-state` holds a single rfc3339 timestamp = the `updated_at` of the newest note analyzed. Next pass selects `updated_at > watermark` ASC, LIMIT `lint_batch_max`, advancing steadily. Missing file ⇒ analyze from the beginning (backlog), still capped per pass. `.lint-state` is non-`.md` so the crawl GC ignores it; `lint.md` is explicitly reserved.

**Cost.** One `infer` call per analyzed note, ≤ `lint_batch_max` per pass. The crawl interval × batch size bounds steady-state spend; first-run backlog amortizes across passes.

**LLM client in the watcher.** `run_crawl_watcher` only holds config + pool; the lint builds its own client via `llm::build_client(&config.llm)` (same pattern as the inbox watcher), skipping cleanly if it can't.

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.
- `cargo test lint` -- expected: parser + renderer unit tests pass.

**Manual checks:**
- With `[wiki] enabled=true lint_enabled=true` and a working LLM backend, capture two deliberately contradictory notes, run `anansi_wiki_lint`, confirm `lint.md` lists the contradiction and `anansi_get` shows `has_conflicts=1` on the affected note; run a crawl and confirm `lint.md` survives.

## Suggested Review Order

**The lint pass (start here)**

- Entry point — single-flight, keyset cursor, per-note loop, append report.
  [`lint.rs:91`](../../src/lint.rs#L91)
- Incremental work-list: keyset pagination on `(updated_at, id)` (review-driven fix).
  [`db.rs:269`](../../src/db.rs#L269)
- Concurrency guard so crawl-lint and the MCP tool don't double-spend.
  [`lint.rs:37`](../../src/lint.rs#L37)
- Robust JSON extraction — balanced braces, ignores trailing prose (review-driven).
  [`lint.rs:244`](../../src/lint.rs#L244)

**Findings → surfaces**

- Conflict flag set/clear (clear nulls the stamp).
  [`db.rs:250`](../../src/db.rs#L250)
- Append-mode `lint.md` so history accumulates (review-driven).
  [`lint.rs:317`](../../src/lint.rs#L317)
- Per-note prompt + neighbor context.
  [`lint.rs:203`](../../src/lint.rs#L203)
- The four-dimension JSON contract.
  [`lint.txt`](../../prompts/lint.txt)

**Wiring**

- Crawl runs the lint phase after the mechanical pass.
  [`crawl.rs:40`](../../src/crawl.rs#L40)
- On-demand `anansi_wiki_lint` MCP tool.
  [`mcp.rs:606`](../../src/mcp.rs#L606)
- `lint.md` reserved from crawl orphan-GC.
  [`wiki.rs:166`](../../src/wiki.rs#L166)

**Peripherals**

- Config gates (default off).
  [`config.rs:213`](../../src/config.rs#L213)

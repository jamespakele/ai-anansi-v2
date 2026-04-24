---
title: 'Anansi v2 — Build 02: Pipeline & Engine'
type: 'feature'
created: '2026-04-23'
status: 'done'
baseline_commit: 'NO_VCS'
context:
  - docs/anansi-v2-spec.md
  - docs/anansi-v2-build-02-pipeline.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The anansi v2 pipeline does not exist. Without the LLM client, prompt assembler, writer, merger, and orchestrator, the three-pass ingest flow (TOC extraction → node expansion → relationship extraction) cannot run.

**Approach:** Implement all pipeline engine modules per `docs/anansi-v2-build-02-pipeline.md` in dependency order: prompt files → llm.rs → prompt.rs → writer.rs → merger.rs → pipeline.rs. Add one missing DB helper (`find_contribution_by_source_toc`) to `db.rs`. Update `lib.rs` to declare new modules.

## Boundaries & Constraints

**Always:**
- `sha2 = "0.10"` must be added to `Cargo.toml` for content-hash.
- Prompt files embedded via `include_str!` — no runtime file loading for prompts.
- `LlmClient` is a `#[async_trait]` trait; `OllamaClient` is the only v1 concrete implementation. `build_client()` returns `Box<dyn LlmClient>`.
- `atomic_write` (write to `.tmp`, rename) for ALL vault file writes.
- Note files must emit YAML frontmatter containing at minimum `anansi_id`, `entity_type`, `name`, `match_key`, and all identity field key-value pairs — this is how `merger.rs` reads existing field values back on subsequent ingest.
- `merge_pure_atomic`: fill blank/`[not mentioned]` fields; record `conflict` in `source_contributions.payload` JSON for differing non-blank values; do NOT overwrite existing filled fields.
- `merge_source_bound`: keyed by `(source_id, toc_address)` via `source_contributions`; if content_hash unchanged → noop; if changed → regenerate file + update note row.
- Source-bound lookup: query `source_contributions` by `(source_id, toc_address)` to find the existing `note_id`, then read the `notes` row.
- Outline match_key: `"outline:{source-slug}"`. Outline contribution uses `toc_address = "0"` sentinel.
- Pass 1 skip: if `anansi_toc` key present in source frontmatter, validate it per spec §9; if valid, skip the LLM call entirely.
- TOC validation fallback: structural validation failure → log reason → fall back to Pass 1.

**Ask First:**
- Any additional DB columns (e.g., a `fields_json` column on `notes`) beyond what the schema in `migrations/0001_schema.sql` provides.
- Any dependency version change from what is already in `Cargo.toml`.

**Never:**
- Implement `mcp.rs`, `main.rs` — those are build-03.
- Add Dockerfile, Cowork plugin, FTS5, embeddings.
- Use `sqlx::query!` / `sqlx::query_as!` compile-time macros (no DATABASE_URL at build time).

## I/O & Edge-Case Matrix

| Scenario | Input | Expected Output | Error Handling |
|----------|-------|-----------------|----------------|
| Pass 1 skip | Source with valid `anansi_toc` frontmatter | No LLM call for Pass 1; TOC parsed from frontmatter | — |
| Invalid preprocessed TOC | Source with malformed `anansi_toc` (duplicate address, unknown entity_type) | Fall back to Pass 1; log warning | — |
| Pure-atomic first write | New person entity | Note file created; DB row inserted; contribution `created` | — |
| Pure-atomic fill blank | Person exists with blank email; new source has email | Email filled; contribution `filled_fields` | — |
| Pure-atomic conflict | Person exists with email A; new source says email B | Both values in payload JSON; contribution `conflict`; file not changed | — |
| Source-bound new | Context node, no prior row for (source_id, toc_address) | File created; DB row inserted; contribution `created` | — |
| Source-bound same hash | Re-ingest unchanged source | Noop; no file write; no new contribution | — |
| Source-bound changed hash | Re-ingest with edited source body | File overwritten; note row updated; contribution `regenerated` | — |
| Container roster union | Org with 2 existing members; new source adds 1 new + 1 duplicate | 1 row added; 1 skipped by dedup; edge written for added row | — |

</frozen-after-approval>

## Code Map

- `Cargo.toml` -- add `sha2 = "0.10"`
- `src/lib.rs` -- add `pub mod` for llm, prompt, writer, merger, pipeline
- `src/db.rs` -- add `find_contribution_by_source_toc(pool, source_id, toc_address)` helper
- `prompts/pass-1-toc-extraction.md` -- Pass 1 prompt template (~120 lines)
- `prompts/pass-3-node-expansion.md` -- Pass 3 prompt template with JSON output schema
- `prompts/pass-4-relationship-extraction.md` -- Pass 4 prompt template, edge JSON array output
- `src/llm.rs` -- `LlmClient` trait, `OllamaClient`, `InferOpts`, `build_client()` (~120 lines)
- `src/prompt.rs` -- `build_pass1/3/4()`, `inject()` helper, `include_str!` prompt constants (~180 lines)
- `src/writer.rs` -- `write_atomic_note`, `write_outline`, `render_roster_section`, `render_entities_section`, `atomic_write`, `render_body` (~200 lines)
- `src/merger.rs` -- `merge_note`, `MergeOutcome`, three strategy fns, `read_note_fields_from_file` (~220 lines)
- `src/pipeline.rs` -- `IngestContext`, `IngestResult`, `ingest()`, `parse_source_file`, `parse_toc`, `validate_preprocessed_toc`, `derive_implicit_edges` (~260 lines)
- `tests/pipeline_integration.rs` -- mock `LlmClient`, 5-leaf source file, full `ingest()` run, DB + file assertions

## Tasks & Acceptance

**Execution:**
- [x] `Cargo.toml` -- add `sha2 = "0.10"` dependency
- [x] `src/lib.rs` -- add `pub mod llm; pub mod prompt; pub mod writer; pub mod merger; pub mod pipeline;`
- [x] `src/db.rs` -- add `find_contribution_by_source_toc(pool, source_id, toc_address) -> Result<Option<(SourceContributionRecord, NoteRecord)>>` using JOIN on source_contributions + notes
- [x] `prompts/pass-1-toc-extraction.md` -- create per build-02 §1: injects `{RULES:Atomicity}`, `{RULES:Downstream-Flow}`, `{ENTITY_TYPES}`, leaf format spec, source body; plain text output only
- [x] `prompts/pass-3-node-expansion.md` -- create per build-02 §2: injects `{RULES:Atomicity}`, `{TEMPLATE_FIELDS}`, `{SOURCE_HINT}`, `{LEAF_HINT}`, `{CONTEXT_AT}`, `{ENTITY_TYPE}`, `{ENTITY_NAME}`, `{TOC_ADDRESS}`, `{SOURCE}`; JSON output schema
- [x] `prompts/pass-4-relationship-extraction.md` -- create per build-02 §3: injects `{IMPLICIT_EDGES}`, `{NODES}`, `{TOC}`, `{RELATIONSHIP_TYPES}`; JSON array output
- [x] `src/llm.rs` -- implement `InferOpts`, `LlmClient` trait, `OllamaClient` (HTTP to `/api/generate`, JSON mode via `"format":"json"`, `ping()` via `/api/tags`), `build_client()`; stub comment for codex-cli/claude-cli
- [x] `src/prompt.rs` -- implement `build_pass1/3/4()` with `inject()` helper; resolve `{RULES:Name}` from RuleRegistry, `{ENTITY_TYPES}` from TemplateRegistry atomic_types, `{TEMPLATE_FIELDS}` from template.field_blocks; unit tests for each build fn with fixture data
- [x] `src/writer.rs` -- implement YAML frontmatter emission (fields as key-value in frontmatter, not just identity metadata), `render_body` with `\{` unescape, `write_atomic_note`, `write_outline`, `render_roster_section` (row_format substitution), `render_entities_section` (wikilinks), `atomic_write`
- [x] `src/merger.rs` -- implement `MergeOutcome` enum, `merge_note` dispatcher, `merge_pure_atomic` (read frontmatter fields, fill-or-conflict logic), `merge_container` (identity merge + roster set-union + member edges), `merge_source_bound` (lookup by source+toc_address, noop/create/regenerate); unit tests for each strategy using in-memory SQLite + tempdir
- [x] `src/pipeline.rs` -- implement `IngestContext`, `IngestResult`, `TocLeaf`, `Pass3Output`, `ingest()` orchestrator, `parse_source_file` (YAML frontmatter split), `parse_toc` (spec §9 regex), `validate_preprocessed_toc` (6 structural checks → fallback signal), `derive_implicit_edges` (4 structural edge types per spec §11)
- [x] `tests/pipeline_integration.rs` -- mock `LlmClient` returning canned JSON for each call; 5-leaf hand-authored source file (mix of pure-atomic, container, source-bound types); assert: 1 source row, 1 outline note, 5 atomic notes, correct edge count; assert Pass 1 skipped when `anansi_toc` present in frontmatter

**Acceptance Criteria:**
- Given the full codebase, when `cargo check` is run, then it exits 0 with no errors
- Given the full codebase, when `cargo test --lib` is run, then all unit tests pass (including new prompt.rs and merger.rs tests)
- Given a 5-leaf mock source file and a mock LlmClient, when `ingest()` is called in the integration test, then the DB contains 1 source row, 1 outline note row, 5 atomic note rows, and at least 5 edges (outline→contains×5), all files exist on disk
- Given a source file with a valid `anansi_toc` frontmatter block, when `ingest()` is called with a call-counting mock LlmClient, then Pass 1 LLM call count is 0 (Pass 3 and Pass 4 still called)
- Given a source with an invalid `anansi_toc` (e.g., duplicate address), when `ingest()` is called, then the pipeline falls back to Pass 1 and the validation warning appears in the return or log

## Design Notes

**Field values in frontmatter:** Note files must store identity fields as YAML key-value pairs in frontmatter (not only in the rendered body). This is how `merger.rs` reads existing field values on subsequent ingest — parsing the markdown body backwards is fragile. Example note frontmatter:
```yaml
---
anansi_id: <uuid>
entity_type: person
name: Ian Kitajima
match_key: person:ian-kitajima
contact_email: ian@example.com
contact_phone: "[not mentioned]"
summary: Research director at PICHTR.
---
```

**Re-ingest architecture:** On every call to `ingest()`, the pipeline MUST first call `find_source_by_path(pool, source_path_str)` to check for a prior source record for this file. Three cases: (1) prior record found AND `content_hash` matches → return early noop; (2) prior record found AND hash differs → create new source record, but pass `prior_source_id = existing.id` to `merge_source_bound` for its contribution lookup; (3) no prior record → fresh ingest, `prior_source_id = None`. `merge_source_bound` takes an `Option<&str>` `prior_source_id` argument and uses that (if Some) instead of `source_id` for `find_contribution_by_source_toc`. This ensures the regeneration path is reachable.

**Roster row dedup:** `parse_roster_from_file` returns existing rows as `{"_row": "raw line text"}`. For dedup, render each NEW row using the row_format template to get its rendered text, then compare against the existing `_row` texts. New rows whose rendered text matches any existing `_row` are skipped. This avoids the structural dedup key mismatch.

**Roster row parsing:** `merge_container` reads existing roster rows by scanning from the `render_as` heading (e.g., `## People`) to the next `##` heading or EOF, splitting at `^- `.

**`derive_implicit_edges` rules (spec §11):**
1. `outline → contains → leaf_note` for every leaf
2. Leaf under org section → `member_of` edge (parent org has roster)
3. Context/event leaf whose address is under event address → `belongs_to` edge
4. Task leaf with `— {assignee}` suffix → `assigned_to` edge (find/create the assignee person note)

## Spec Change Log

**Loop 1 — 2026-04-24:** bad_spec — re-ingest source_id architecture. `ingest()` generated a new UUID every call, making `find_contribution_by_source_toc(new_source_id, ...)` unable to find prior contributions, and `insert_source` crashing on duplicate `content_hash`. Fix: added `find_source_by_path` to db.rs; `ingest()` now calls it first and passes `prior_source_id` to `merge_source_bound`. KEEP: llm.rs, prompt.rs, Pass 1 skip/fallback, parse_toc, parse_source_file, write_atomic_note structure, merger None/create paths, integration test structure.

## Verification

**Commands:**
- `cargo check` -- expected: exits 0, no errors
- `cargo test --lib` -- expected: all unit tests pass
- `cargo test` -- expected: all tests including integration pass (requires tempdir writes)

## Suggested Review Order

**Core contracts**

- `LlmClient` trait + `InferOpts` — the capability boundary the whole pipeline depends on
  [`llm.rs:24`](../../src/llm.rs#L24)

- `OllamaClient` + `build_client()` — the only v1 concrete impl; `"format":"json"` mode wiring
  [`llm.rs:91`](../../src/llm.rs#L91)

**Pipeline orchestration**

- `ingest()` — entry point: `find_source_by_path` noop guard, Pass 1→3→4 sequence, `prior_source_id` threading
  [`pipeline.rs:484`](../../src/pipeline.rs#L484)

- `IngestContext` + `TocLeaf` — parameter bundles that cross every module boundary
  [`pipeline.rs:43`](../../src/pipeline.rs#L43)

- `parse_toc` — regex-based leaf parser; address format, entity_type extraction, hint capture
  [`pipeline.rs:199`](../../src/pipeline.rs#L199)

- `validate_preprocessed_toc` — 6 structural checks that trigger Pass 1 fallback
  [`pipeline.rs:269`](../../src/pipeline.rs#L269)

**Merge strategies**

- `MergeOutcome` — enum contract for all three merge paths
  [`merger.rs:15`](../../src/merger.rs#L15)

- `merge_pure_atomic` — fill-blank / conflict logic; reads existing frontmatter to compare
  [`merger.rs:108`](../../src/merger.rs#L108)

- `merge_container` — identity merge + roster set-union via rendered-row string comparison
  [`merger.rs:222`](../../src/merger.rs#L222)

- `merge_source_bound` — `prior_source_id` lookup path; noop / create / regenerate branches
  [`merger.rs:472`](../../src/merger.rs#L472)

- `read_frontmatter_fields` — how merger reads existing field values back from disk
  [`merger.rs:25`](../../src/merger.rs#L25)

**Vault writes**

- `write_atomic_note` — YAML frontmatter emission order; field sorting for stability
  [`writer.rs:66`](../../src/writer.rs#L66)

- `render_body` — `{{field}}` substitution with `\{` literal-brace unescape
  [`writer.rs:188`](../../src/writer.rs#L188)

**Prompt assembly**

- `build_pass1` / `build_pass3` / `build_pass4` — inject-based template assembly
  [`prompt.rs:57`](../../src/prompt.rs#L57)

**DB helpers**

- `find_source_by_path` — ORDER BY ingested_at DESC enables re-ingest noop guard
  [`db.rs:131`](../../src/db.rs#L131)

- `find_contribution_by_source_toc` — JOIN query that enables source-bound regeneration
  [`db.rs:297`](../../src/db.rs#L297)

**Tests & peripherals**

- Integration test — MockLlm dispatch; 3-scenario coverage (basic, toc-skip, toc-fallback)
  [`pipeline_integration.rs:8`](../../tests/pipeline_integration.rs#L8)

- `Cargo.toml` — `sha2 = "0.10"` addition (only new dependency)
  [`Cargo.toml:1`](../../Cargo.toml#L1)

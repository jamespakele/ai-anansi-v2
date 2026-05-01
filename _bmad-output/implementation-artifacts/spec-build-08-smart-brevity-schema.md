---
title: 'Anansi v2 — Build 08: Smart Brevity Schema & Structs'
type: 'feature'
created: '2026-04-30'
status: 'review'
baseline_commit: ''
context:
  - docs/HANDOFF-anansi-smart-brevity.md
---

<frozen-after-approval reason="human-owned intent — renegotiated 2026-04-30: file_path removed; content_toc/content_sb collapsed to single content field; variant (toc/sb) is pipeline-level via source provenance, not schema-level">

## Intent

**Problem:** The current schema stores notes with `summary_1`/`summary_5` (generic field names), requires `file_path NOT NULL` (a pointer to a web/ vault file), and has no columns for inline note content, conflict tracking, or embeddings. All code — DB queries, the NoteRecord struct, MCP tools, and the pipeline — references these old field names and the file-pointer pattern. This blocks the Smart Brevity inline-content pipeline.

**Approach:** (1) Write `migrations/0002_smart_brevity.sql`: DROP TABLE notes and CREATE it fresh with `lede`, `why`, `content` (single field), `has_conflicts`, `conflicts_updated_at` — no `file_path`, no `summary_1`/`summary_5`, no `content_toc`/`content_sb`. Create `embeddings` table. (2) Update `NoteRecord` struct: remove `file_path`, rename `summary_1→lede`/`summary_5→why`, add `content`, `has_conflicts`, `conflicts_updated_at`. Update all DB queries. (3) Update `mcp.rs` — `anansi_search` searches `name`/`lede`/`why`; `anansi_get` returns `content` inline (no file reading). (4) Update `pipeline.rs`: rename field references, remove file-writing calls, update Pass-3 LLM prompt to use `"lede"`/`"why"` JSON keys. (5) Update `merger.rs`: remove all `file_path`-based file reads/writes. (6) Update prompt templates for `lede`/`why` key names.

> **No data migration.** All existing note data and web/ files are discarded. Source files are tracked in `sources.source_path` only — notes have no file path.

> **Single `content` field.** The distinction between TOC-pipeline and SB-pipeline content is a pipeline-level concern, not a schema concern. Which pipeline produced a note's content is tracked via `source_contributions` (source provenance). Entity notes (person, org, concept, area) only come from the TOC pipeline; discussion notes (chapter, section) may come from either pipeline. The `content` field holds whatever the pipeline wrote — no variant columns needed.

## Boundaries & Constraints

**Always:**
- No data migration — DROP TABLE notes and CREATE fresh; all existing note data and files are discarded
- `cargo check` and `cargo test --lib` must pass after all changes
- `file_path` is removed from `notes` entirely — not made nullable, not kept
- Single `content` field replaces `content_toc`/`content_sb` — the variant is tracked via source provenance, not on the note
- The `embeddings` table is created but NOT populated by this build — embedding logic is Phase 3
- Migration must be wrapped in `BEGIN TRANSACTION; ... COMMIT;` and bracketed with `PRAGMA foreign_keys = OFF/ON`

**Ask First:**
- Any change to `merger.rs` merge strategies beyond field renames and file-op removal
- Note: removing `path: PathBuf` from `MergeOutcome::Created` and `MergeOutcome::Regenerated` is approved (D2 resolution — no file is created in this build)
- Any change to template files or `template.rs`

**Never:**
- Populate `content` during the old pipeline — it stays NULL until Build-09
- Add embedding logic — only create the table
- Change edge schema or edge-related queries

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Fresh DB | `anansi init` on empty dir | 0001 + 0002 migrations run; `notes` table has `lede`, `why`, `content`, `has_conflicts`, `conflicts_updated_at` — no `file_path`; `embeddings` table exists | — |
| `anansi_search` query "Ian" | Search with keyword | Matches against `name`, `lede`, `why` | Empty result if no match |
| `anansi_get` note with `content` set | `content IS NOT NULL` | Returns note metadata + `"content"` inline | — |
| `anansi_get` note with `content IS NULL` | Note not yet atomized (Build-09 pending) | Returns note metadata + `"warning": "content_not_available"` | No error; structured warning in response |
| Old pipeline ingest | Ingest via existing `pipeline.rs` | `lede` populated from LLM `"lede"` key, `why` from `"why"` key; `content` stays NULL; no file written | — |

</frozen-after-approval>

## Code Map

- `migrations/0001_schema.sql` -- current schema (reference only — do not modify)
- `migrations/0002_smart_brevity.sql` -- [NEW] DROP TABLE notes + CREATE fresh (single `content`, no file_path); CREATE TABLE embeddings
- `src/db.rs:67` -- `NoteRecord` struct; remove `file_path`, rename `summary_1→lede`/`summary_5→why`, add `content`, `has_conflicts`, `conflicts_updated_at`
- `src/db.rs:170` -- `insert_note`; remove `file_path`; rename summary columns; add new columns in INSERT + ON CONFLICT
- `src/db.rs:214` -- `row_to_note`; remove `file_path` mapping; rename summary mappings; add new field mappings
- `src/db.rs:231` -- `update_note_file_path`; DELETE this function entirely
- `src/db.rs:316` -- `search_notes`; update WHERE clause to 3 LIKE binds: `name`, `lede`, `why`
- `src/db.rs:351` -- `find_contribution_by_source_toc`; update SQL aliases and row mappings for renamed fields
- `src/mcp.rs:142` -- `anansi_search` tool description; update field names
- `src/mcp.rs:288,308` -- `tool_search`; rename JSON response key `"summary_1"→"lede"` (line 288); remove `"file_path": n.file_path` (line 308 — compile error after field removal)
- `src/mcp.rs:331,345,358,359,360` -- `tool_get`; remove file-reading at line 345 (`tokio::fs::read_to_string(&note.file_path)`); remove `"file_path": note.file_path` at line 358; rename `"summary_1": note.summary_1` → `"lede": note.lede` at line 359 and `"summary_5": note.summary_5` → `"why": note.why` at line 360; return `content` inline; return warning when `content IS NULL`
- `src/pipeline.rs:37-38` -- `Pass3Output` fields `summary_1`/`summary_5`; rename to `lede`/`why`
- `src/pipeline.rs:374-384` -- `parse_pass3_response_from_value`; read `"lede"` and `"why"` keys only
- `src/pipeline.rs:534-538` -- `build_pass4_input`; update node summary label string literal `"summary_1"` → `"lede"`
- `src/pipeline.rs:687-689` -- outline note creation; remove `file_path` at line 687, rename `summary_1→lede` at line 688, `summary_5→why` at line 689
- `src/pipeline.rs:800-812` -- note prototype + `render_fields` injection; remove `file_path`; rename struct literal field assignments `summary_1: Some(p3_out.summary_1.clone())` → `lede: Some(p3_out.lede.clone())` and `summary_5: Some(p3_out.summary_5.clone())` → `why: Some(p3_out.why.clone())` at lines 800-801; rename string literal map keys `render_fields.insert("summary_1", ...)` → `render_fields.insert("lede", ...)` and `render_fields.insert("summary_5", ...)` → `render_fields.insert("why", ...)` at lines 811-812
- `src/pipeline.rs:1060-1080` -- tests; remove `file_path`, rename field names and JSON keys
- `src/merger.rs:123,134,255,262,278,488,509` -- `PathBuf::from(&note.file_path)` / `std::fs::read_to_string(&note.file_path)` call sites; remove all
- `src/merger.rs:124,154,246,360,489,510` -- `writer::write_atomic_note(...)` call sites; remove all
- `src/merger.rs:135,263` -- `read_frontmatter_fields(&path)` call sites; remove — also remove dependent block using `existing_fields` (~lines 136-165 in `merge_pure_atomic`, ~lines 263-273 in `merge_container`)
- `src/merger.rs:153,332` -- `read_note_body(&path)` call sites; remove
- `src/merger.rs:25` -- `read_frontmatter_fields` function definition; delete entirely (dead code after call sites removed)
- `src/merger.rs:456` -- `read_note_body` function definition; delete entirely (dead code after call sites removed)
- `src/merger.rs:563` -- `let path = vault.atomic_note_path(entity_type, name)` in `make_note_proto`; delete line — unused variable after `file_path` field assignment removed
- `src/merger.rs:561-577` -- `make_note_proto` test helper; remove `file_path` field assignment (line 569), delete `let path` at line 563, rename `summary_1: None` → `lede: None` and `summary_5: None` → `why: None`
- `src/merger.rs:599` -- `MergeOutcome::Created { note_id, path }` test match arm; remove `path` binding and `assert!(path.exists())` assertion — compile error after `path: PathBuf` removed from variant
- `src/merger.rs:1-14` -- `use` block; remove `use crate::writer;`, `use std::path::Path;`, `use std::path::PathBuf;` (or `use std::path::{Path, PathBuf};`) — unused after file-op removal
- `src/writer.rs:66-97` -- delete `write_atomic_note` function only; preserve all other functions: `write_outline`, `render_roster_section`, `render_entities_section`, `render_body`, `yaml_value` — `pipeline.rs:699` still calls `write_outline`
- `src/pipeline.rs:863-871` -- `MergeOutcome::Created { note_id, .. }` and `Regenerated { note_id, .. }` match arms; use `..` wildcard already — will compile after `path` field removed; verify no explicit `path` binding present
- `tests/pipeline_integration.rs` -- contains mock LLM JSON string literals `"summary_1"` and `"summary_5"` (not NoteRecord struct construction); rename to `"lede"` and `"why"` — runtime test failure if missed, not compile error
- `prompts/pass-3-node-expansion.md:38,44,60,61,71,72` -- rename `summary_1→lede`, `summary_5→why` in JSON keys and rules
- `prompts/pass-3-batch.md:63,64,87,88` -- rename `summary_1→lede`, `summary_5→why` in JSON keys and rules
- `prompts/pass-4-relationship-extraction.md:9` -- update node format comment from `summary_1` to `lede`
- `src/prompt.rs:125` -- doc comment referencing `summary_1`; update to `lede`

## Tasks & Acceptance

**Execution:**

- [x] `migrations/0002_smart_brevity.sql` -- [NEW] Write migration:
  ```sql
  PRAGMA foreign_keys = OFF;
  BEGIN TRANSACTION;

  DELETE FROM source_contributions;
  DELETE FROM edges;
  DROP TABLE IF EXISTS notes;
  CREATE TABLE notes (
      id                   TEXT    PRIMARY KEY,
      entity_type          TEXT    NOT NULL,
      name                 TEXT    NOT NULL,
      match_key            TEXT    NOT NULL UNIQUE,
      lede                 TEXT,
      why                  TEXT,
      content              TEXT,
      has_conflicts        INTEGER NOT NULL DEFAULT 0,
      conflicts_updated_at TEXT,
      merge_category       TEXT    NOT NULL DEFAULT '',
      created_from         TEXT    NOT NULL,
      source_count         INTEGER NOT NULL DEFAULT 0,
      created_at           TEXT    NOT NULL,
      updated_at           TEXT    NOT NULL,
      FOREIGN KEY (created_from) REFERENCES sources(id)
  );
  CREATE INDEX idx_notes_match_key    ON notes (match_key);
  CREATE INDEX idx_notes_entity_type  ON notes (entity_type);
  CREATE INDEX idx_notes_has_conflicts ON notes (has_conflicts);

  DROP TABLE IF EXISTS embeddings;
  CREATE TABLE embeddings (
      id          TEXT    PRIMARY KEY,
      note_id     TEXT    NOT NULL,
      model       TEXT    NOT NULL,
      dimensions  INTEGER NOT NULL,
      vector      BLOB    NOT NULL,
      created_at  TEXT    NOT NULL,
      UNIQUE (note_id, model),
      FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
  );

  COMMIT;
  PRAGMA foreign_keys = ON;
  ```

- [x] `src/db.rs` -- Update `NoteRecord` struct: remove `file_path` entirely; rename `summary_1: Option<String>` → `lede: Option<String>`, `summary_5: Option<String>` → `why: Option<String>`; add `content: Option<String>`, `has_conflicts: i64`, `conflicts_updated_at: Option<String>`. Update `insert_note` SQL: remove `file_path`, rename summary columns, add new columns; ON CONFLICT clause must be: `ON CONFLICT(match_key) DO UPDATE SET lede=COALESCE(lede, excluded.lede), why=COALESCE(why, excluded.why), content=COALESCE(content, excluded.content), source_count=source_count+1, updated_at=excluded.updated_at`. Update `row_to_note`: remove `file_path` mapping, rename summary mappings, add new field mappings. Update `search_notes` WHERE clause to `name LIKE ? OR lede LIKE ? OR why LIKE ?` with exactly 3 `.bind(&pattern)` calls (SQLx panics at runtime if bind count mismatches). Update `find_contribution_by_source_toc`: rename SQL aliases `n.summary_1→n.lede`, `n.summary_5→n.why`; remove `file_path` from SELECT; add `n.content`, `n.has_conflicts`, `n.conflicts_updated_at` to SELECT (function returns full NoteRecord — all columns must be present or row mapping panics at runtime); update row mappings to match. Delete `update_note_file_path` function at `~line 231`.

- [x] `src/mcp.rs` -- Update `tool_search` (line 288): rename JSON key `"summary_1"→"lede"`; remove `"file_path": n.file_path` from JSON response at line 308 (compile error after field removal); update tool description at line 142. Update `tool_get` (lines 331, 345, 358, 359, 360): remove all file-reading logic (line 345); remove `"file_path"` from response (line 358); rename `"summary_1"→"lede"` and `"summary_5"→"why"` in response (lines 359-360); return `"content"` inline. Full response shape: `{"id": ..., "entity_type": ..., "name": ..., "match_key": ..., "lede": ..., "why": ..., "content": ..., "has_conflicts": ..., "conflicts_updated_at": ..., "merge_category": ..., "source_count": ..., "created_at": ..., "updated_at": ...}`. When `content IS NULL`, include `"warning": "content_not_available", "message": "note has no inline content yet"` as sibling keys in the same JSON object alongside the metadata fields above.

- [x] `src/pipeline.rs` -- Rename `Pass3Output` fields `summary_1→lede`, `summary_5→why` (lines 37-38). Update `parse_pass3_response_from_value` (lines 374-384): read `"lede"` and `"why"` JSON keys only — no fallback. Update `build_pass4_input` (lines 534-538): rename comment `summary_1→lede` at line 534; rename field access `p3.summary_1→p3.lede` at line 538 (compile error if missed). Update outline note creation (lines 687-689): remove `file_path` at line 687, rename struct fields at lines 688-689. Update note prototype and `render_fields` injection (lines 800-812): remove `file_path`; rename struct literal field assignments `summary_1→lede` and `summary_5→why` at lines 800-801; rename string literal map keys `render_fields.insert("summary_1", ...)→render_fields.insert("lede", ...)` and `render_fields.insert("summary_5", ...)→render_fields.insert("why", ...)` at lines 811-812. Update all test JSON and assertions (lines 1060-1080): rename JSON string keys and field accesses.

- [x] `prompts/pass-3-node-expansion.md` + `prompts/pass-3-batch.md` + `prompts/pass-4-relationship-extraction.md` -- Rename all `summary_1→lede` and `summary_5→why` occurrences in JSON example keys, rules text, and field descriptions. In `pass-4-relationship-extraction.md`, update the node format comment from `summary_1` to `lede`. These are `include_str!`'d by `prompt.rs` — failing to rename here means the LLM returns `"summary_1"` but the parser reads `"lede"` → blank fields.

- [x] `src/prompt.rs` -- Update doc comment at line 125 from `summary_1` to `lede`.

- [x] `src/merger.rs` -- Build-08 scope: make it compile after NoteRecord field changes. (1) Remove `PathBuf::from` and `std::fs::read_to_string` call sites using `file_path` (lines 123, 134, 255, 262, 278, 488, 509). (2) Remove all `writer::write_atomic_note(...)` calls (lines 124, 154, 246, 360, 489, 510). (3) Remove all `read_frontmatter_fields(...)` call sites (lines 135, 263) AND the entire blocks that use the result (`existing_fields`) — approximately lines 136-165 in `merge_pure_atomic` and lines 263-273 in `merge_container`; these blocks will not compile once `existing_fields` is undefined. After removal, `MergeOutcome::FilledFields` and `MergeOutcome::Conflict` become unreachable outcomes from these functions — this is intentional; full merger redesign is deferred to the atomize parser build. (4) Remove all `read_note_body(...)` call sites (lines 153, 332). (5) Delete `read_frontmatter_fields` function definition at line 25 and `read_note_body` function definition at line 456 (dead code after call site removal). (6) Remove `use crate::writer;`, `use std::path::Path;`, `use std::path::PathBuf;` from the import block (lines 1-14) — unused after file-op removal. (7) Update `MergeOutcome::Created` and `MergeOutcome::Regenerated` variants: remove `path: PathBuf` field. Caller `pipeline.rs:863-871` already uses `..` wildcard — will continue to compile; verify no explicit `path` binding there. (8) Update `make_note_proto` test helper (lines 561-577): delete `let path = vault.atomic_note_path(...)` at line 563 (becomes unused), remove `file_path: path.to_string_lossy().to_string()` at line 569, rename `summary_1: None → lede: None` and `summary_5: None → why: None`. (9) Fix test match arm at line 599: remove `path` binding from `MergeOutcome::Created { note_id, path }` and delete `assert!(path.exists())`. Note: the full SB-pipeline merger redesign is deferred to the atomize parser build.

- [x] `src/writer.rs` -- Delete only the `write_atomic_note` function body (lines 66-97). Preserve all other functions: `write_outline`, `render_roster_section`, `render_entities_section`, `render_body`, `yaml_value` — `pipeline.rs:699` still calls `write_outline` and `pipeline.rs:813` still calls `render_body`; deleting the whole file would break these.

**Acceptance Criteria:**

- Given `cargo check`, then exits 0 with no errors
- Given `cargo test`, then all existing tests pass (with `file_path` removed and fields renamed)
- Given a fresh `anansi init`, when inspecting the DB schema, then `notes` table has columns `lede`, `why`, `content`, `has_conflicts`, `conflicts_updated_at` — and NO `file_path`, `summary_1`, `summary_5`, `content_toc`, or `content_sb` — and `embeddings` table exists
- Given `anansi_get` on a note with `content` set, then response includes `"content"` field inline
- Given `anansi_get` on a note with `content IS NULL`, then response includes `"warning": "content_not_available"`
- Given a Pass-3 LLM response with `"lede"` and `"why"` JSON keys, then `Pass3Output.lede` and `Pass3Output.why` are populated correctly
- Given a full ingest run via the old pipeline (Pass 3 + Pass 4), then all inserted `notes` rows have `content IS NULL`

### Review Findings

**Decisions resolved:**

- [x] [Review][Decision→Patch] `parse_pass3_response_from_value` — no `summary_1` fallback. Read `"lede"` and `"why"` keys only; update Pass-3 LLM prompt to match.
- [x] [Review][Decision→Patch] `anansi_get` content-unavailable state — return structured warning when `content IS NULL`.
- [x] [Review][Decision→Patch] File path/pointer pattern — eliminated entirely from notes.
- [x] [Renegotiation] `content_toc`/`content_sb` → single `content` field. The toc/sb variant is a pipeline-level concern tracked via source provenance (`source_contributions`), not a schema-level concern. SB pipeline only produces discussion/chapter content; entity notes (person, org, concept, area) come exclusively from TOC pipeline.

**Patches applied:**

- [x] [Review][Patch] `merger.rs` 7 production `file_path` call sites added to Code Map [`merger.rs:123,134,255,262,278,488,509`]
- [x] [Review][Patch] `writer.rs:72` added to Code Map with compile-fix task
- [x] [Review][Patch] Migration wrapped in `BEGIN TRANSACTION`/`COMMIT` and `PRAGMA foreign_keys = OFF/ON`
- [x] [Review][Patch] Migration FK constraint included in DDL
- [x] [Review][Patch] `find_contribution_by_source_toc` SQL aliases and row mappings both specified
- [x] [Review][Patch] `update_note_file_path` function marked for deletion
- [x] [Review][Patch] `search_notes` bind count callout added (SQLx runtime panic risk)
- [x] [Review][Patch] Prompt template files (`prompts/pass-3-*.md`, `prompts/pass-4-*.md`) added to Code Map and tasks

**Deferred — pre-existing, not caused by Build-08:**

- [x] [Review][Defer] `baseline_commit` blank
- [x] [Review][Defer] No down migration / rollback strategy
- [x] [Review][Defer] `source_contributions` UNIQUE(source_id, note_id) design concern

#### Round 2 (2026-04-30)

**Decisions needed:**

- [x] [Review][Decision→Dismiss] match_key design note — sentence is correct forward-looking design: `sb:slug` vs `toc:slug` will distinguish the two pipelines on the same source when SB pipeline is implemented. No spec change needed.
- [x] [Review][Decision→Patch] merger.rs scope clarified — `lede` + `why` + `content` are all populated together by the future SB atomized-file parser, not by Build-08. Merger redesign is deferred to the SB parser build. Build-08 merger scope = make it compile: remove file_path-derived path construction, remove `write_atomic_note` calls (dead code — no path to construct), remove `read_frontmatter_fields`/`read_note_body` calls, remove `path: PathBuf` from `MergeOutcome::Created` and `MergeOutcome::Regenerated`. The "file→content substitution" language in the current task is wrong — file ops drop out with no replacement in this build.

**Patches needed:**

- [x] [Review][Patch] Migration orphaned FK rows — add `DELETE FROM source_contributions;` and `DELETE FROM edges;` before `DROP TABLE IF EXISTS notes` inside the transaction; clean slate leaves orphaned rows in child tables otherwise [`migrations/0002_smart_brevity.sql`]
- [x] [Review][Patch] writer.rs task too vague — "update so it compiles" must specify: delete `write_atomic_note` if it has no callers after merger.rs changes; if callers remain, remove only the `note.file_path` field reference [`src/writer.rs:72`]
- [x] [Review][Patch] tool_search also emits `"file_path"` — mcp.rs task at line 288 says rename `"summary_1"→"lede"` but does not mention removing `"file_path": n.file_path` at line 308; this is a compile error after field removal [`src/mcp.rs:308`]
- [x] [Review][Patch] render_fields string literal keys — pipeline.rs task does not explicitly call out renaming `render_fields.insert("summary_1", ...)` → `render_fields.insert("lede", ...)` and `render_fields.insert("summary_5", ...)` → `render_fields.insert("why", ...)` at lines 811-812 [`src/pipeline.rs:811-812`]
- [x] [Review][Patch] build_pass4_input label — pipeline.rs task does not explicitly call out updating the format label string from `"summary_1"` to `"lede"` in `build_pass4_input` [`src/pipeline.rs:534-538`]
- [x] [Review][Patch] tests/pipeline_integration.rs missing from Code Map — constructs NoteRecord with `file_path`; will fail after field removal [`tests/pipeline_integration.rs`]
- [x] [Review][Patch] AC uses cargo test --lib — integration tests also exist and must pass; change to `cargo test` [AC #2]
- [x] [Review][Patch] anansi_search content scope unacknowledged — nowhere does the spec state that `anansi_search` intentionally does not search `content`; add explicit acknowledgment as deferral to Build-09 [`src/mcp.rs:288`]
- [x] [Review][Patch] merger.rs write_atomic_note call sites absent — Code Map lists only 7 `file_path` lines but omits `write_atomic_note` calls at lines 154, 246, 360; also omits `read_frontmatter_fields` (lines 135, 263) and `read_note_body` (lines 153, 332) which are the file-read helpers being removed [`src/merger.rs:154,246,360,135,263,153,332`]
- [x] [Review][Patch] false pipeline.rs:~830 reference — no `write_atomic_note` call exists in pipeline.rs; all write calls are in merger.rs; remove this Code Map entry [`src/pipeline.rs:~830`]

#### Round 3 (2026-04-30)

**Decisions needed:**

- [x] [Review][Decision→Patch] `insert_note` ON CONFLICT clause — coalesce: `ON CONFLICT(match_key) DO UPDATE SET lede=COALESCE(lede, excluded.lede), why=COALESCE(why, excluded.why), content=COALESCE(content, excluded.content), source_count=source_count+1, updated_at=excluded.updated_at` — never overwrites existing values, fills NULLs only.

**Patches needed:**

- [x] [Review][Patch] MergeOutcome callers in pipeline.rs missing from spec — pipeline.rs:863-871 pattern-matches `MergeOutcome::Created { note_id, .. }` and `Regenerated { note_id, .. }`; removing `path` field from variants means these match arms compile fine (use `..` wildcard) but must be verified [`src/pipeline.rs:863-871`]
- [x] [Review][Patch] merger.rs test at line 599 explicitly binds path — `MergeOutcome::Created { note_id, path }` with `assert!(path.exists())` — compile error and broken assertion after removing `path: PathBuf` from variant; remove `path` binding and `path.exists()` assertion [`src/merger.rs:599`]
- [x] [Review][Patch] mcp.rs tool_get has additional file_path and field rename sites not in Code Map — line 345: `tokio::fs::read_to_string(&note.file_path)`, line 358: `"file_path": note.file_path`, lines 359-360: `"summary_1": note.summary_1` / `"summary_5": note.summary_5` — all compile errors [`src/mcp.rs:345,358,359,360`]
- [x] [Review][Patch] writer.rs task ambiguity — "delete write_atomic_note entirely" could be read as "delete the file"; clarify: delete only the `write_atomic_note` function (lines 66-97); preserve `write_outline`, `render_roster_section`, `render_entities_section`, `render_body`, `yaml_value` — `pipeline.rs:699` still calls `write_outline` [`src/writer.rs`]
- [x] [Review][Patch] pipeline.rs Code Map 688-689 misses line 687 — `file_path: outline_path.to_string_lossy().to_string()` is at line 687, not covered by the 688-689 range; update Code Map entry to 687-689 [`src/pipeline.rs:687`]
- [x] [Review][Patch] pipeline.rs task underspecifies lines 800-801 struct literal — lines 800-801 contain `summary_1: Some(p3_out.summary_1.clone())` and `summary_5: Some(p3_out.summary_5.clone())` — compile errors distinct from the render_fields string literals at 811-812; add explicit call-out [`src/pipeline.rs:800-801`]
- [x] [Review][Patch] merger.rs function definitions become dead code — `read_frontmatter_fields` defined at line 25 and `read_note_body` defined at line 456 will be orphaned dead code after all call sites are removed; delete both function definitions [`src/merger.rs:25,456`]
- [x] [Review][Patch] merger.rs identity-merge block depends on removed read_frontmatter_fields — lines ~136-165 in `merge_pure_atomic` and ~263-273 in `merge_container` use `existing_fields` initialized by `read_frontmatter_fields`; removing the call site alone leaves `existing_fields` undefined — compile error; must also remove the entire block using it (filled/conflicts logic) [`src/merger.rs:136-165,263-273`]
- [x] [Review][Patch] merger.rs make_note_proto unused variable — line 563 `let path = vault.atomic_note_path(entity_type, name)` will be an unused variable after line 569 `file_path: path.to_string_lossy()` is removed; delete line 563 too [`src/merger.rs:563`]
- [x] [Review][Patch] merger.rs unused imports after file-op removal — `use crate::writer;`, `use std::path::{Path, PathBuf}` become unused after removing all write_atomic_note/read_frontmatter_fields/read_note_body calls; add explicit cleanup of these imports to the merger.rs task [`src/merger.rs:1-14`]
- [x] [Review][Patch] mcp.rs tool_get complete response shape unspecified — task says "return `content` inline; add `lede`, `why`, `has_conflicts`, `conflicts_updated_at`" but does not enumerate the full response fields (id, entity_type, name, match_key, etc.); and does not confirm the warning is a sibling key in the same JSON object as metadata (not a nested envelope) [`src/mcp.rs:331`]
- [x] [Review][Patch] HANDOFF doc still references content_toc/content_sb — sections 7.1, 7.4, 7.5, 10.1, 10.2 of the context doc still describe two content fields; anansi_search SQL shows 5 binds (name, lede, why, content_toc, content_sb); any developer reading HANDOFF as context will implement the wrong schema [`docs/HANDOFF-anansi-smart-brevity.md`]
- [x] [Review][Patch] tests/pipeline_integration.rs Code Map description wrong — spec says "constructs NoteRecord with file_path" but the file contains mock LLM JSON string literals (`"summary_1"`, `"summary_5"`) not NoteRecord struct construction; rename the JSON keys to `"lede"`/`"why"` (runtime test failure if missed, not compile error) [`tests/pipeline_integration.rs`]
- [x] [Review][Patch] find_contribution_by_source_toc SELECT must cover all new NoteRecord fields — function returns NoteRecord; task says rename summary aliases but does not say to also SELECT the three new columns (`content`, `has_conflicts`, `conflicts_updated_at`); missing columns cause a runtime panic on row mapping [`src/db.rs:351`]
- [x] [Review][Patch] No AC for content staying NULL after old-pipeline ingest — "Never: Populate content during old pipeline" is a constraint but has no corresponding testable AC; add AC: given a full ingest run via old pipeline, then `notes.content IS NULL` for all inserted rows [`## Acceptance Criteria`]

## Design Notes

**Why DROP + CREATE instead of ALTER TABLE:** No data migration needed — all existing notes are discarded. DROP + CREATE is simplest and sidesteps SQLite's limited DDL. `PRAGMA foreign_keys = OFF/ON` bracket required because `source_contributions` and `edges` hold FK references to `notes(id)`.

**Why no `file_path` on notes:** Notes are atomic knowledge units, not file pointers. Smart Brevity blocks are 150-300 words — small enough to store inline. Source files tracked in `sources.source_path`.

**Why single `content` instead of `content_toc`/`content_sb`:** The variant (toc vs sb) depends on the source type and user preference — board meetings may get TOC treatment, community meetings SB, hobby meetings both. Hardcoding two columns bakes in exactly two variants and implies every note could have both. A single `content` field lets each note hold whatever the pipeline wrote. The variant provenance is in `source_contributions` (which source → which note). Entity notes (person, org, concept, area) only come from TOC pipeline; discussion notes (chapter, section) may come from either. If both pipelines run on the same source, the match_key can include the variant to distinguish them.

**Pass-3 prompt update:** Rename `summary_1`/`summary_5` → `lede`/`why` in prompt templates. No backward compat fallback — prompt and parser updated together. Safe since nothing is in production.

**`has_conflicts` index:** Indexed for fast conflict notification query (`WHERE has_conflicts = 1`).

**`anansi_search` does not search `content`:** Intentional. The `content` field holds full prose blocks (150-300+ words) from the atomize pipeline. Full-text search over it would return false positives and swamp keyword results. Search covers `name`, `lede`, `why` only. A dedicated content-search tool belongs to the atomize parser build.

**`content` is NULL after old-pipeline ingest:** The old pipeline does not populate `content`. Notes ingested via the old pipeline will have `lede` and `why` but no inline content. `anansi_get` returns `"warning": "content_not_available"`. Build-09 populates content.

## Verification

**Commands:**
- `cargo check` -- expected: exits 0, no errors
- `cargo test` -- expected: all tests pass
- `sqlite3 test.db ".schema notes"` -- expected: columns include `lede`, `why`, `content`, `has_conflicts`, `conflicts_updated_at`; NO `file_path`, `summary_1`, `summary_5`, `content_toc`, or `content_sb`
- `sqlite3 test.db ".schema embeddings"` -- expected: table exists with `id`, `note_id`, `model`, `dimensions`, `vector`, `created_at`; UNIQUE(note_id, model); FK to notes(id) ON DELETE CASCADE

---
title: 'Anansi v2 — Build 12: LLM-Wiki Dual-Write Layer (WikiStore)'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '97b3953'
context:
  - _bmad-output/implementation-artifacts/spec-build-11-inbox-watcher.md
  - output/llm-wiki-compress-toc.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Anansi's knowledge lives only in PostgreSQL. There is no file-native, human-browsable, Obsidian-compatible projection of the graph that an LLM (or person) can read, link, and reason over locally — the Karpathy "LLM-wiki" pattern. The file-writing machinery (`writer.rs`, `vault.rs`, `export.rs`) was scaffolded but the live capture path never writes notes to disk.

**Approach:** Add a `WikiStore` that **dual-writes**: every successful capture continues to write Postgres as today (Postgres stays the source of truth), and immediately afterward projects the affected notes from canonical DB state into Karpathy-style markdown under `/data/llm-wiki/` — one `{slug}.{entity_type}.md` file per note with YAML frontmatter and `[[wikilink]]` connections, plus a maintained `index.md` catalog and an append-only `log.md`. Gated behind a new optional `[wiki]` config section (default off → zero behavior change). This is Goal A of the LLM-Wiki epic; the async maintenance crawl (Goal B) and tipping-point eviction (Goal C) are deferred (see `deferred-work.md`).

## Boundaries & Constraints

**Always:**
- Postgres is the source of truth. Wiki files are a **projection** materialized from canonical post-merge DB rows (read back by id after the upsert), never the authority.
- Wiki writes happen only **after** the DB write for that note succeeds (dual-write order: DB → project).
- Wiki write failures are logged to stderr and **non-fatal** — they must never roll back or fail a DB ingest/capture.
- All wiki file writes are atomic (tmp-file + rename), reusing the existing slug/wikilink conventions from `vault.rs`.
- Backward compatible: the `[wiki]` config section is optional and defaults to `enabled = false`. When absent or disabled, no wiki files are written and existing behavior is byte-for-byte unchanged.
- `index.md` and `log.md` follow the Karpathy conventions: `index.md` is a catalog grouped by entity_type (`- [[slug.ext|Name]] — lede`); `log.md` is append-only with parseable prefixes (`## [YYYY-MM-DD] ingest | <title> (+N notes)`).

**Ask First:**
- Whether note **removal** on `anansi_delete_note` / `anansi_archive_note` is in scope now, or left to Goal B's crawl to garbage-collect orphan wiki files. (Spec currently includes a cheap `remove_note` call on delete/archive.)
- Whether `index.md` is rebuilt in full from all notes each materialize (simple, correct, O(N)) vs incrementally patched. (Spec uses full rebuild.)

**Never:**
- Do not make the wiki authoritative or read-through; reads still hit Postgres/MCP.
- Do not implement the async crawl/lint (Goal B) or size-based eviction / last-access tracking (Goal C).
- Do not introduce a `NoteStore` trait threaded through every call site — the projection-writer approach achieves dual-write with far less churn (see Design Notes).
- Do not block the Tokio event loop on large wiki writes beyond the existing per-ingest cost.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Atomized ingest, wiki enabled | `.md` ingested via queue/MCP/inbox | N note files + outline file at `/data/llm-wiki/`, `index.md` rebuilt, `log.md` appended | N/A |
| Single capture, wiki enabled | `anansi_capture` upsert | One `{slug}.{type}.md` written/overwritten, index + log updated | N/A |
| Wiki disabled (default) | no `[wiki]` or `enabled=false` | No files written; DB path unchanged | N/A |
| Wiki write fails | `/data/llm-wiki` unwritable | DB ingest still returns `ok`; error logged `[wiki] ...` | Caught, logged, swallowed |
| Re-ingest revised note | same `match_key`, new content | Wiki file overwritten with canonical merged content from DB | N/A |
| Delete / archive note | `anansi_delete_note` | Corresponding wiki file removed (best-effort) | Missing file ignored |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- add `WikiConfig { enabled: bool, dir: String }` (default `enabled=false`, `dir="/data/llm-wiki"`); add `#[serde(default)] pub wiki: WikiConfig` to `Config` + `Config::default()`. Mirror the `InboxConfig` pattern.
- `src/wiki.rs` -- NEW. `WikiStore { root: PathBuf, enabled: bool }`; `from_config(&Config)`; `materialize(&self, pool, note_ids: &[String], source_title: &str)` (reads canonical notes+edges from DB, writes files, rebuilds index, appends log); `write_note(note, edges, name_lookup)`; `remove_note(entity_type, name)`; private `render_note_markdown`, `rebuild_index`, `append_log`; atomic write helper.
- `src/lib.rs` -- add `pub mod wiki;`.
- `src/db.rs` -- add `all_note_summaries(pool) -> Vec<(entity_type, name, match_key, lede)>` for the index rebuild (read-only).
- `src/atomized_ingest.rs` -- add final param `wiki: Option<&WikiStore>` to `ingest_atomized`; after the edge passes, collect all written note ids (outline + `address_index` values) and call `wiki.materialize(pool, &ids, &source_title)` (non-fatal).
- `src/queue.rs` -- build `WikiStore::from_config(&config)` in `run_queue_watcher`, thread through `scan_and_ingest` → `process_queued_file` → `ingest_atomized(..., Some(&wiki))`. This single site covers BOTH the queue-direct path AND the inbox pipeline (inbox Stage 3 only *enqueues* a file to `/data/q-atomize/`; the queue watcher is the sole `ingest_atomized` consumer — `inbox.rs` needs no change).
- `src/mcp.rs` -- `tool_ingest_atomized` needs NO wiki wiring: it only writes the file to `q-atomize/` and returns `queued`; the queue watcher is the sole `ingest_atomized` consumer and materializes there. Wire `tool_capture` (materialize the single upserted note), and `tool_delete_note`/`tool_archive_note` (`remove_note` + `refresh_index`).
- `anansi.toml.example` -- append commented `[wiki]` section documenting `enabled` and `dir`.

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- add `WikiConfig` struct (`enabled: bool`, `dir: String`) with serde defaults `default_wiki_dir()="/data/llm-wiki"`, `enabled=false`; add `#[serde(default)] pub wiki: WikiConfig` to `Config` and `wiki: WikiConfig::default()` to `Config::default()`.
- [x] `src/wiki.rs` -- implement `WikiStore` per Code Map. `materialize`: for each id, `db::get_note` + `db::edges_for_note`; resolve each edge's other-note name/entity_type (via `db::get_note`) to build `[[slug.ext|Name]]` wikilinks; render frontmatter (`anansi_id`, `entity_type`, `name`, `match_key`, `updated_at`) + body (`# name`, lede, `*why*`, content) + `## Connections` (mirror `export.rs` format); `atomic_write` to `{root}/{slug}.{entity_type}.md`; then `rebuild_index` from `db::all_note_summaries`; then `append_log`. All steps wrapped so any error logs `[wiki]` and returns `Ok(())`. No-op when `!enabled`.
- [x] `src/lib.rs` -- add `pub mod wiki;`.
- [x] `src/db.rs` -- add `all_note_summaries(pool: &DbPool) -> Result<Vec<NoteSummary>>` (SELECT entity_type, name, match_key, lede FROM notes ORDER BY entity_type, name).
- [x] `src/atomized_ingest.rs` -- add `wiki: Option<&WikiStore>` param to `ingest_atomized`; collect `note_ids` (outline_note_id + address_index values); after Pass 2c, `if let Some(w) = wiki { let _ = w.materialize(pool, &note_ids, &parsed.source_title).await; }`.
- [x] `src/queue.rs` -- construct `WikiStore::from_config(&config)` once in `run_queue_watcher`; thread `&WikiStore` through `scan_and_ingest`/`process_queued_file`; pass to `ingest_atomized`. (Covers the inbox path too — inbox enqueues; the queue watcher ingests. No `inbox.rs` change.)
- [x] `src/mcp.rs` -- wire `WikiStore` into `tool_capture` (materialize single note id), `tool_delete_note` + `tool_archive_note` (`remove_note` + `refresh_index`). `tool_ingest_atomized` left unchanged (it enqueues; the queue watcher materializes).
- [x] `anansi.toml.example` -- append commented `[wiki]` section.
- [x] `src/wiki.rs` (tests) -- unit-test the I/O matrix: render produces valid frontmatter + `[[wikilink]]`; disabled store is a no-op; missing-file removal is silent; YAML-unsafe names/ledes are quoted.

**Acceptance Criteria:**
- Given `cargo check`, when run after all changes, then output is `Finished` with zero errors.
- Given no `[wiki]` section in `anansi.toml`, when the server runs an ingest, then no files appear under `/data/llm-wiki/` and DB output is identical to pre-Build-12.
- Given `[wiki] enabled = true` and an atomized file ingested, when the queue watcher processes it, then `/data/llm-wiki/` contains one `{slug}.{entity_type}.md` per note (with YAML frontmatter and `## Connections` wikilinks), a rebuilt `index.md`, and a `log.md` line `## [<date>] ingest | <title> (+N notes)`.
- Given `/data/llm-wiki/` is read-only at runtime, when an ingest runs, then the ingest still returns `status: ok`, notes are in Postgres, and stderr shows a `[wiki]` error.
- Given a note is re-ingested with revised content, when materialized, then its wiki file body reflects the canonical merged DB content.

## Spec Change Log

- **v1.1 (review patches, 2026-06-24):** Three-reviewer adversarial pass. No intent_gap/bad_spec — no loopback. Patched: (1) `tests/pipeline_integration.rs` `Config` literal missing the new `wiki` field broke `cargo test` — added `wiki: WikiConfig::default()`; (2) `wiki.rs::atomic_write` used a fixed `.tmp` name — concurrent `index.md` writers (queue task + MCP handlers) could corrupt it; switched to a unique `<pid>.<seq>` tmp so the final rename is last-writer-wins on a complete file; (3) `index.md` listed archived notes and delete/archive didn't refresh it → dangling wikilinks — `all_note_summaries` now excludes `archive-%`, materialize skips archived edge targets, and delete/archive call new `WikiStore::refresh_index`; (4) `materialize` per-note loop now logs-and-continues instead of `?`-aborting, so one bad note no longer leaves `index.md` un-rebuilt. Corrected Code Map/Task drift: `tool_ingest_atomized` enqueues (no wiki wiring); the queue watcher is the sole `ingest_atomized` consumer. Deferred (see deferred-work.md): raw-`entity_type`-as-extension traversal (shared with Build-05 item), `already_ingested` skipping self-heal (Goal B crawl), `update_note` not wiki-wired (Goal B crawl), O(N) index rebuild (revisit at Goal C scale). KEEP: projection-writer architecture, post-merge canonical read-back, non-fatal dual-write, the `export.rs`-mirrored file format.

## Design Notes

**Why projection-writer, not a `NoteStore` trait.** The original intent framed this as "subclasses that override the DB-write methods." Rust has no subclassing, and—more importantly—**dual-write makes a trait swap the wrong tool.** A trait (`PgStore`/`WikiStore` swapped into every call site) fits *replacement*; here the wiki is a *buffer/projection* of Postgres that must stay authoritative. Materializing from canonical DB rows *after* each write is (a) correct-by-construction — it serializes post-COALESCE-merge state, which the in-memory `NoteRecord` doesn't reflect; (b) low-churn — `process_block` and all inline SQL stay untouched; (c) a direct reuse of the `export.rs` precedent (DB→markdown). The single convergence point `ingest_atomized` covers the MCP tool, queue watcher, and inbox pipeline at once.

**Wiki file format (mirrors `export.rs`):**
```markdown
---
anansi_id: <uuid>
entity_type: person
name: Ian Kitajima
match_key: person:ian-kitajima
updated_at: 2026-06-24T...
---

# Ian Kitajima

<lede>

*<why>*

<content>

## Connections

- [[pichtr.organization|PICHTR]] `works_at`
```
Files: notes at `{root}/{slug}.{entity_type}.md`, catalog at `{root}/index.md`, journal at `{root}/log.md`. Slug/wikilink helpers reuse the `vault.rs` algorithm (`slug_name`).

**Conscious duplication.** `render_note_markdown` intentionally duplicates `export.rs`'s frontmatter/connections format rather than extracting a shared serializer now — the two differ in link format (typed `[[slug.ext|name]]` vs `export`'s name-only) and extracting a resolver-parameterized renderer is out of scope for the foundation. Dedup is a candidate for a later cleanup pass.

**Container path.** `/data` is the vault root inside Docker (`CMD anansi2 serve --root /data`); `/data/llm-wiki/` is already persisted by the existing `./data:/data` bind mount — no compose change needed.

## Verification

**Commands:**
- `cargo check` -- expected: `Finished`, zero errors (pre-existing dead_code warnings acceptable).
- `cargo test wiki` -- expected: all `src/wiki.rs` unit tests pass.
- `docker compose up --build -d && docker compose logs anansi -f` -- expected: server starts; with `[wiki] enabled=true`, after an ingest `ls /data/llm-wiki/` shows note files + `index.md` + `log.md`.

**Manual checks:**
- Set `[wiki] enabled = true`, drop a `.md` into `/data/q-atomize/`, confirm wiki files appear with valid frontmatter and resolvable `[[wikilinks]]` (open the folder in Obsidian; graph view should connect the notes).

## Suggested Review Order

**The design seam (start here)**

- Entry point — the whole feature is one projection writer; read this first.
  [`wiki.rs:45`](../../src/wiki.rs#L45)
- Pure renderer — the on-disk format (frontmatter + body + typed `## Connections`), mirrors `export.rs`.
  [`wiki.rs:210`](../../src/wiki.rs#L210)
- Construction from config gate — `enabled`/`dir` decide whether anything is written.
  [`wiki.rs:34`](../../src/wiki.rs#L34)

**Dual-write wiring (where writes are projected)**

- Ingest convergence — one post-pass call covers queue + inbox + MCP-ingest; non-fatal.
  [`atomized_ingest.rs:258`](../../src/atomized_ingest.rs#L258)
- Queue watcher threads the store; sole `ingest_atomized` consumer.
  [`queue.rs:51`](../../src/queue.rs#L51)
- Capture path materializes the single upserted note after the DB write.
  [`mcp.rs:1280`](../../src/mcp.rs#L1280)
- Delete/archive remove the file + refresh the catalog (no dangling links).
  [`mcp.rs:1369`](../../src/mcp.rs#L1369)

**Correctness hot-spots (review-driven)**

- Unique `<pid>.<seq>` tmp — prevents concurrent `index.md` corruption.
  [`wiki.rs:324`](../../src/wiki.rs#L324)
- Archived notes excluded from the index catalog.
  [`db.rs:213`](../../src/db.rs#L213)
- `refresh_index` — keeps the catalog consistent on removal paths.
  [`wiki.rs:110`](../../src/wiki.rs#L110)

**Peripherals**

- Config: optional `[wiki]` section, default off (backward compatible).
  [`config.rs:196`](../../src/config.rs#L196)
- Read query backing the index rebuild.
  [`db.rs:213`](../../src/db.rs#L213)

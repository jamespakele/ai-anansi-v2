# Deferred Work

Items surfaced during review but not caused by the current change, or scoped to a later build.

---

## From Web Tender Review (Build-10)

**Web Tender Phase 2 — 6 additional maintenance checks**

Scoped to a future build after the initial 9 checks are deployed and stable. See `docs/web-tender-handoff.md` for full spec context.

| # | Check | Confidence | Behavior |
|---|-------|-----------|----------|
| 1 | **Index/outline sync** | High (auto-fix) | Verify `index.md` and per-type outline notes list every entity that exists. Add missing entries, remove entries for deleted notes. |
| 2 | **Normalization rules** | High (auto-fix) | Read rules from `%Rules/normalization.md` and apply them (name casing, punctuation, slug format). Rules can change between runs; the tender adapts. |
| 3 | **Template alignment** | High (auto-fix) | Compare each note's fields against the current template for its `entity_type`. If fields are missing, renamed, or in the wrong format, re-atomize the note through the current `sb-atomize` pipeline. |
| 4 | **Source integrity** | Medium (flag) | Verify every note's `source_id` still exists in the `sources` table. Flag orphans. |
| 5 | **Edge inference** | Medium (flag) | Scan note `content` for mentions of known entities (by name or match_key) and suggest missing edges. |
| 6 | **Semantic deduplication** | Low (flag) | Use vector similarity (embedding cosine distance) to find near-duplicates. Flag pairs above a configurable threshold for review. |

**Dependencies:**
- Check #3 (template alignment) depends on the `sb-atomize` pipeline being available at runtime
- Check #6 (semantic dedup) depends on `note_embeddings` table being populated (embedding generation must be running)

**outline path contradiction (§4 vs §8/§11)**
Spec §4 (authoritative folder layout) says outlines live at `web/{source-slug}.outline.md`. Spec §8 and §11 say `outlines/{source-slug}.md` (a subfolder). `vault.rs` follows §4. Before build-02 pipeline implementation, the spec author should confirm §4 is canonical and update §8/§11 to match.

**TOCTOU race on find_note_by_match_key + insert_note**
A concurrent `find → None → insert` sequence from two async tasks on the same entity will cause the second `insert_note` to fail with a UNIQUE constraint violation on `match_key`. Build-02 pipeline should use an upsert or serialize note writes with a per-entity lock/queue.

**path traversal on source_slug and toc_address**
`vault.rs` path helpers apply `slug_name()` to the `name` parameter but not to `source_slug` or `toc_address`. If these values ever come from untrusted input (LLM output or user-provided filenames), a crafted value like `../../etc/passwd` could write outside the vault. Build-02/03 should sanitize these at pipeline entry.

**empty slug → hidden/colliding filenames**
`slug_name("")` or `slug_name("!!!")` returns `""`, producing filenames like `-.md` or `.md`. Build-02 pipeline should validate that entity names produce non-empty slugs before calling vault path helpers.

**%% delimiter not anchored to line start**
`parse_field_blocks` uses `remaining.find("%%\n")` which matches `%%` anywhere in a line (e.g., `50%%`). All current templates use `%%` only at line start, so this is safe today. Revisit if templates are ever authored by external tools.

**wikilink display name containing `|` or `]]`**
`vault.rs::wikilink()` doesn't sanitize the display name. A name containing `|` or `]]` would produce a broken Obsidian wikilink. Build-02 or a utility function should escape/replace these characters when writing wikilinks.

---

## From Build-02 Review

**No transaction wrapping `ingest()`**
`ingest()` interleaves DB writes, LLM calls, and file writes across Pass 1→3→4 with no enclosing transaction. A crash mid-pipeline leaves partial state (some notes written, edges missing, outline absent). Consider wrapping DB mutations in a single transaction or adding a `rollback_source(source_id)` cleanup path.

**Concurrent `atomic_write` tmp-file collision**
All atomic writes use a fixed `.tmp` extension (e.g., `foo.md.tmp`). Two concurrent `ingest()` calls on the same note will race on the tmp file. Use a unique tmp suffix (e.g., process ID + timestamp) or a per-file lock.

**`yaml_value` doesn't handle YAML bare keywords**
`yaml_value("true")`, `yaml_value("null")`, `yaml_value("1234")` emit unquoted values that YAML parsers interpret as bool/null/integer, not strings. LLM-generated field values may match these patterns. Quote strings that look like YAML scalars.

**LLM-generated frontmatter key names not sanitized**
`write_atomic_note` emits LLM-supplied field keys directly as YAML key names. A key containing `:`, `#`, `\n`, or a space would produce malformed frontmatter. Sanitize or allowlist field key names before writing.

**`parse_frontmatter_fields` over-strips quotes**
`trim_matches('"')` removes all leading/trailing quotes rather than one pair, so `"""foo"""` becomes `foo`. Use a single-pair strip (check first and last char both `"`, then slice `1..len-1`).

**Single-segment TOC address collision with outline sentinel `"0"`**
`parse_toc` accepts `"0"` as a valid address (it matches `\d+`), which aliases the outline sentinel used by `merge_source_bound`. Guard against `toc_address == "0"` in `validate_preprocessed_toc` and `parse_toc`.

**`find_contribution_by_source_toc` missing `LIMIT 1`**
The query joins `source_contributions` and `notes` without `LIMIT 1`. If the UNIQUE constraint ever relaxes or a data migration creates duplicates, `fetch_optional` will error. Add `LIMIT 1` for defensive correctness.

**`render_roster_section` leaves unreplaced `{placeholder}` patterns silently**
If a row's `HashMap` is missing a key referenced in `row_format`, the `{placeholder}` text is emitted verbatim. Log a warning or replace with `[not mentioned]` to surface template mismatches during development.

---

## From Build-03 Review

**`tool_ingest` with `source_path` leaks filesystem path in error message**
When `pipeline::ingest` fails on a caller-supplied `source_path`, the full server-side path is included in the JSON-RPC error string returned to the client. For a locally-hosted single-user instance this is fine, but sanitize before any multi-user or network-exposed deployment.

**`tool_get` silently omits file content when `file_path` is relative and CWD differs from vault root**
`tokio::fs::read_to_string(&note.file_path)` uses the process working directory if `file_path` is a relative path. If the binary is started from a directory other than the vault root, the read silently fails (`.ok()` swallows the error) and the response omits the `content` field with no indication to the caller. Normalize `file_path` to absolute at note-write time, or join against `anansi_root` at read time.

**`cmd_init` leaves partially-initialized vault on TOML parse error**
`cmd_init` writes directories and seed files before calling `Config::load`. If the embedded `anansi.toml.example` is malformed TOML, `Config::load` errors after the filesystem is partially modified (dirs + templates exist, DB absent). Add a cleanup or an early validation step.

**Dockerfile runs as root**
The runtime image has no `USER` directive; the `anansi2` process runs as root. Create a non-root user (`adduser --system anansi`) and switch with `USER anansi` before `ENTRYPOINT` to reduce blast radius from any code-execution vulnerability.

**`anansi_search` empty query string returns all rows**
`%{query}%` with an empty string becomes `%%`, which matches every row. No lower bound on query length is enforced. Add a minimum length check (e.g., ≥1 character) or return an empty result for blank queries.

---

## From Build-04 Review

**Integration test hardcodes pre-Build-04 flat layout**
`tests/pipeline_integration.rs` constructs `Vault::new(root, Path::new("web"))` and asserts the outline at `root/web/` — the old flat structure. The test does not exercise the new `anansi/web/` layout and would not catch regressions in the `anansi/` subdir path resolution. Update the integration test's `make_context()` to use the new default paths (`"anansi/web"` etc.) or derive paths from a `Config` loaded from a temp vault initialised by `cmd_init`.

**Docker volume rename is a data-loss footgun for existing deployments**
Build-04 renamed `VOLUME ["/anansi"]` to `VOLUME ["/vault"]` and `./anansi:/anansi` to `./vault:/vault` in docker-compose.yml. Users upgrading an existing Docker deployment will silently start writing to a fresh `/vault` volume while their data remains at `./anansi` — no error, no warning. Add an upgrade note to README.md and/or a startup check that warns if `/anansi` exists but `/vault/anansi/anansi.toml` does not.

---

## From Build-05 Review

**`summary_1`/`summary_5` collision risk in render_fields injection**
`pipeline.rs` injects `summary_1` and `summary_5` into the cloned `p3_out.fields` map before calling `render_body`. If a future template declared an `identity_field` named `summary_1` or `summary_5`, the pipeline injection would silently overwrite the LLM-provided value. Currently no templates use these names. Guard against this by using `entry().or_insert()` instead of `insert()`, so LLM-provided values always win.

---

**`entity_type` value used directly in atomic note filenames without sanitization**
`vault::atomic_note_path` uses `entity_type` directly as the file extension (`{slug}.{entity_type}.md`). Entity types that come from LLM-generated Pass 3 JSON (`EntityRef.entity_type`, pipeline.rs:369) are not validated against the template registry or restricted to the `[A-Za-z_?]+` character class that the TOC text parser enforces. A crafted or hallucinated `entity_type` containing `/` passed directly to `atomic_note_path` or `wikilink` could write outside `vault.web` or produce malformed wikilinks. Add validation that `entity_type` matches `^[A-Za-z_]+$` before constructing typed paths.

---

## Deferred from: code review of spec-build-08-smart-brevity-schema (2026-04-30)

- `baseline_commit` is blank — Code Map line numbers can't be verified against a pinned state. Edge Hunter confirmed most numbers are accurate at HEAD; process gap only.
- No down migration / rollback strategy for local DB recovery after a failed 0002 run. Pre-existing SQLx limitation; the table-drop step is irreversible without a backup. Out of scope for Build-08.
- `content_sb` NULL-for-resource-notes invariant has no DB-level enforcement (no CHECK constraint). Application-level "Never" constraint in the spec covers it for Build-08. Enforcement belongs to the atomized ingest pipeline (Build-09).
- `source_contributions` UNIQUE(source_id, note_id) blocks multiple TOC contributions from the same source to the same note at different TOC addresses. Pre-existing design constraint; not introduced by Build-08. Revisit when Build-09 atomized ingest is implemented.

---

## Deferred from: LLM-Wiki epic split (2026-06-24)

The LLM-Wiki layer was split into three independently-shippable goals (decision: `[S]` Split, build foundation first). **Build-12 (this spec) covers Goal A only** — the `NoteStore`/`WikiStore` trait and dual-write of Karpathy-style markdown to `/data/llm-wiki/`. The following goals depend on Build-12 and are deferred to later builds:

**Goal B — `anansi-crawl` (the lint/maintenance pass), depends on A.** Reframed (per James) away from a naive "sync loop" toward a Karpathy-style **lint** operation that performs wiki maintenance AND treats Postgres-reconciliation as just one check among several. A periodic background task (mirror the `queue.rs`/`inbox.rs` watcher pattern) that, ordered by last-access date, walks the wiki and: (1) **verifies wiki↔Postgres sync** — since Build-12 dual-writes, every wiki note should already have a matching `notes` row; the crawl flags/repairs drift (missing rows, stale content, orphaned files); (2) refreshes `index.md` (the content catalog) and appends `log.md` entries; (3) runs Karpathy lint checks — contradictions, stale claims superseded by newer sources, orphan pages lacking inbound `[[wikilinks]]`, missing cross-references, data gaps. Postgres remains the source of truth; the wiki is a maintained projection/cache of it.

## From Build-14 Review (2026-06-24 — semantic lint)

- **Transient-error notes skip until re-edited.** The lint advances the keyset cursor past any note whose LLM call errored this pass (forward progress, avoids poison-note stalls). Such notes are surfaced in `report.errors` but only re-linted when their own `updated_at` changes. A bounded retry / dead-letter queue is future work.
- **Contradiction flagging is asymmetric.** When note A is found to contradict neighbor B, only A gets `has_conflicts = 1`; B is flagged only when B itself is later analyzed (and only if the model agrees from B's perspective). Future: flag both endpoints when a contradiction names a resolvable neighbor.
- **Stale conflict flags on neighbor change.** A note is re-linted only when *its own* `updated_at` bumps — editing a *neighbor* that resolves a contradiction doesn't clear this note's flag until it's re-analyzed. Future: re-lint a note when an incident edge/neighbor changes.
- **No global LLM budget.** Per-pass cost is bounded by `lint_batch_max`, and steady-state by `× crawl_interval_secs`, but there's no daily/token circuit breaker if `infer` keeps erroring. Future: add a budget/backoff.

## From high-effort branch code-review (2026-06-24 — Build-22 triage)

The `/code-review high` pass over the whole `feat/llm-wiki` branch (45 agents) found a cluster of wiki↔Postgres drift bugs. The default-config correctness ones were fixed in **Build-22** (`tool_update_note`/`tool_relate` re-projection, single-error crawl freeze, caps+crawl-disabled index, export `log.md` line, lint archived-neighbor filter). These are **deferred** (capped-mode-only, perf, or quality):

- **Capped-mode dangling wikilinks (`wiki.rs` crawl, ~L213).** A resident note can render a `## Connections` link to a neighbor the same crawl evicts/GCs → broken Obsidian link. Only with `max_mb`/`max_notes` set. A correct fix needs a two-pass crawl (determine the resident id-set first, then render connections only to resident notes). Defer to a capped-mode-polish build.
- **Eviction budget double-counts same-name `:variant` notes (`wiki.rs` crawl, ~L234).** Two distinct notes with different match_keys (the `:variant` suffix) but the same `name`+`entity_type` map to one filename; the cap loop counts both. Capped-only budget inaccuracy. A real fix changes the wiki file-naming convention (currently `slug(name).entity_type.md`, shared with `vault.rs`) to incorporate the variant — a cross-cutting change. Defer.
- **Warmth ordering mixes timestamp formats (`db.rs::live_note_ids_by_access_desc`).** `last_accessed_at` is fixed-6-digit µs; `updated_at` is chrono AutoSi (variable digits). `ORDER BY COALESCE(last_accessed_at, updated_at) DESC` is a TEXT compare, so warmth can misorder when the two columns' fractional widths differ → under caps the crawl may evict a hotter note. Proper fix: normalize `updated_at`'s format too, or add a numeric warmth column (epoch millis). Capped-only. Defer.
- **Capture awaits an inline full-table-scan index rebuild (`mcp.rs tool_capture`/`materialize`).** Every `anansi_capture` blocks on `get_note` + `SELECT all notes` + full `index.md` rewrite → O(total notes) per capture. The synchronous dual-write is intentional (the wiki is immediately consistent after a capture), but at scale it should move to a background `tokio::spawn` materialize and/or an incremental index update. Defer (perf).
- **Quality / cleanup:** dead `db::live_note_ids_by_access` (Build-13 coldest-first ASC query, zero callers since Build-15 switched to `_desc`); `edge_other`/`resident_cutoff`/JSON-recovery logic duplicated across `wiki.rs`/`lint.rs`/`export.rs`; the deeply-nested `json!({content:[{type:text,text:...}]})` MCP envelope hand-built per tool. Candidates for a `/simplify` pass.

## From Build-15 Review (2026-06-24 — eviction)

- **Index staleness for a note deleted between crawls when capped.** With caps active, `refresh_index` is skipped (the crawl owns the resident index), so a note deleted via `anansi_delete_note` has its file removed immediately but remains listed in `index.md` (a broken wikilink) until the next crawl rebuilds the catalog (≤ `crawl_interval_secs`). Bounded and self-healing; a cap-aware incremental index update would remove the staleness.
- **Soft size cap.** `max_mb` can be exceeded by up to one note's size (file sizes are measured after each write, and the boundary note is admitted). A hard cap would require rendering each note and checking its projected size *before* writing. Acceptable while notes are small; revisit if very large notes appear.
- **Pre-existing: 3 `prompt::tests` failures** (`build_pass1_contains_rules_and_entity_types`, `build_pass3_all_placeholders_replaced`, `build_pass4_all_placeholders_replaced`) fail at HEAD independent of the wiki builds — they load templates/`%Rules` from disk and appear path/environment-dependent. Surfaced incidentally by running the full `cargo test --lib`; out of scope for the LLM-wiki epic. Investigate the test fixtures' file resolution.

**Goal C — tipping-point eviction (LRU cache bound), depends on A + B.** [DONE in Build-15] A configurable size cap on `/data/llm-wiki/`. When exceeded, evict oldest-accessed wiki files first (the wiki is a bounded LRU buffer over Postgres). Cache-miss reads fall through to the Anansi backend (Postgres / MCP server) for content no longer resident in the wiki. Requires last-access tracking (atime is unreliable under Docker `noatime`/`relatime` — likely needs an explicit access-timestamp sidecar or frontmatter field maintained on read). The tipping point may be system-specific; expose it as config with a sane default.

## From Build-12 Review (2026-06-24)

**`entity_type` used raw as the wiki file extension (`{slug}.{entity_type}.md`)** — `wiki.rs::note_path`/`wikilink` mirror `vault::atomic_note_path`, which the Build-05 review already flagged for the same unsanitized-`entity_type` path-traversal risk. `slug_name` neutralizes the `name`, but a hypothetical `entity_type` containing `/` or `..` would escape the wiki root on write/remove. Same fix as the existing Build-05 deferred item (validate `entity_type` matches `^[A-Za-z_]+$`) — apply once, centrally, covering both `vault.rs` and `wiki.rs`. Not exploitable today (entity types come from the template registry / TOC parser charset).

**`already_ingested` dedup short-circuits wiki materialization** — `atomized_ingest::ingest_atomized` returns early on a content-hash match before the dual-write block. So if the wiki dir is wiped, `[wiki] enabled` is newly turned on, or a prior materialize failed, re-feeding identical content will NOT re-project it. This is acceptable for Build-12 because **Goal B's `anansi-crawl` lint is the designated wiki↔Postgres reconciliation / self-heal mechanism** — it rebuilds missing/stale wiki files from canonical state. Track as a Goal B acceptance case.

**`tool_update_note` is not wired to the wiki** — Build-12 wired capture/delete/archive + the ingest path, but not `anansi_update_note`. A note whose `name` or `entity_type` changes via update orphans its old `{slug}.{type}.md` file (no removal, no re-projection). Goal B's crawl garbage-collects orphans; alternatively wire `update_note` to `remove_note(old)` + `materialize(new)` in a focused follow-up.

**`index.md` is fully rebuilt (full notes-table scan + whole-file rewrite) on every capture** — O(N) per write. This was an explicit "Ask First" the spec resolved in favor of full rebuild (simple, correct at Karpathy's moderate scale). Revisit with an incremental index update when the wiki approaches the Goal C tipping point.

## From Web Tender Code Review Fixes (Round 1) Review

**Resolved audit rows accumulate forever**
`audit_auto_fix` inserts a `tender_queue` row (severity='info', status='resolved') for every auto-fix, but there's no retention/cleanup. On a long-running watcher with frequent passes, `tender_queue` grows unboundedly with resolved info rows. Consider a periodic `DELETE FROM tender_queue WHERE status='resolved' AND resolved_at < now() - interval '30 days'` or a retention job.

**Conflicting edges double-count**
`get_conflicting_edges` uses a symmetric WHERE clause, so each conflicting pair yields two rows (e1 and e2). The flag count is therefore 2× the real conflict count, and `insert_or_update_flag`'s dedup (by category only, match_key=None) means the second insert returns Ok(false) and isn't counted. Pre-existing from Build-10, not introduced by the fixes.

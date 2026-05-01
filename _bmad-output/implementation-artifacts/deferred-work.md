# Deferred Work

Items surfaced during review but not caused by the current change, or scoped to a later build.

---

## From Build-01 Review

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

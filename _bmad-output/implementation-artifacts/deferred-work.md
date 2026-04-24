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

---
title: 'Anansi v2 — Build 04: Vault Restructure, Typed Notes & Remember Skill'
type: 'feature'
created: '2026-04-24'
status: 'done'
baseline_commit: 'NO_VCS'
context:
  - docs/anansi-v2-spec.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The vault mixes user sources with anansi-generated artifacts (outlines, typed notes, config, DB) in the same directory; leaf note filenames use an opaque `-{slug}.md` prefix rather than encoding entity type readably; and there is no single-motion "remember" workflow for frontier-model clients (Claude Desktop, ChatGPT/Codex via tunnel) to preprocess a document and trigger anansi storage in one MCP call.

**Approach:** (1) Move all anansi-managed artifacts under an `anansi/` subdirectory so vault root stays clean for user sources. (2) Change leaf filenames from `-{slug}.md` to `{slug}.{entity_type}.md`. (3) Add an `anansi-remember` plugin skill that does TOC preprocessing with entity typing in Claude context and submits the enriched source via `anansi_ingest content+filename` in one MCP call.

## Boundaries & Constraints

**Always:**
- All callers pass `vault_root` to `Config::load`; `Config::load` looks for `vault_root/anansi/anansi.toml` (one-line path change in the function body — no caller changes needed).
- Default path values update to: `web_dir = "anansi/web"`, `rules_dir = "anansi/%Rules"`, `templates_dir = "anansi/templates"`, `db_file = "anansi/web.db"`. Existing path helper methods (`web_path`, `db_path`, `templates_path`, `rules_path`) are unchanged — they already call `root.join(&self.paths.X)`.
- `vault.source_path(slug)` writes sources to `vault.root` (vault-root/slug.md) — unchanged; sources stay in the vault root, never under `anansi/`.
- `vault.outline_path` and `vault.source_bound_path` conventions are unchanged.
- `atomic_note_path(entity_type, name)` new convention: `{slug}.{entity_type}.md`. If `entity_type` is empty, use `"note"` as the extension. All template entity types (person, organization, topic, concept, note, task, area, project, event, context, container, email_thread, meeting_summary, research_paper, action_item_list) use their type string verbatim as the extension.
- `wikilink(entity_type, name)` new format: `[[{slug}.{ext}|{name}]]` using the same extension fallback logic as `atomic_note_path`.
- The `anansi_toc` text format is unchanged — entity_type is already embedded via `[type]` notation per leaf line. No pipeline changes are needed.
- `cargo check` and `cargo test --lib` must pass after all changes.
- The existing `anansi-toc` skill and `/toc` command are left untouched.

**Ask First:**
- Any DB schema changes or new Cargo dependencies.

**Never:**
- Move source files out of vault root into `anansi/`.
- Add typed extensions to `outline_path` or `source_bound_path`.
- Change the `anansi_toc` text format or `validate_preprocessed_toc` logic.
- Alter MCP tool signatures or JSON-RPC protocol.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `anansi2 --root /vault init` (fresh) | empty dir | `/vault/anansi/{web,%Rules,templates}/` created; `anansi/anansi.toml` + `anansi/web.db` written; vault root not polluted with config or DB files | errors propagate with context |
| `anansi2 --root /vault init` (existing) | already-initialised vault | no-op for existing files; migrations run on existing DB | — |
| `atomic_note_path` person | `entity_type="person"`, `name="Ian Kitajima"` | `{web}/ian-kitajima.person.md` | — |
| `atomic_note_path` fallback | `entity_type=""` | `{web}/{slug}.note.md` | — |
| `wikilink` person | `entity_type="person"`, `name="Ian Kitajima"` | `[[ian-kitajima.person\|Ian Kitajima]]` | — |
| `wikilink` fallback | `entity_type=""` | `[[{slug}.note\|{name}]]` | — |
| remember skill — MCP server up | source content + filename; server reachable at configured URL | `anansi_ingest` called with `content + filename`; ingested IDs returned | surface error message to user |
| remember skill — MCP server down | source content + filename; server unreachable | write augmented file (with `anansi_toc` frontmatter) to `vault_root/{filename}`; print `anansi2 --root {root} ingest {path}` command as fallback | — |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- `Config::load` path and four default path functions; `load_example_config` test
- `anansi.toml.example` -- `[paths]` section reflects new `anansi/` layout
- `src/vault.rs` -- `atomic_note_path` and `wikilink` rewritten for typed `{slug}.{type}.md` convention; unit tests updated
- `src/main.rs` -- `cmd_init` directory list and seed destinations moved to `anansi/` subdir
- `plugin/anansi.plugin/skills/anansi-remember/SKILL.md` -- new 8-step remember skill
- `plugin/anansi.plugin/commands/remember.md` -- new /remember slash command
- `plugin/anansi.plugin/.claude-plugin/plugin.json` -- updated name, description, keywords
- `Dockerfile` -- updated `VOLUME` and `CMD` default root from `/anansi` to `/vault`
- `docker-compose.yml` -- updated volume bind and env references

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- in `Config::load`, change `anansi_root.join("anansi.toml")` to `anansi_root.join("anansi").join("anansi.toml")`; change four default path functions: `default_web_dir` → `"anansi/web"`, `default_rules_dir` → `"anansi/%Rules"`, `default_templates_dir` → `"anansi/templates"`, `default_db_file` → `"anansi/web.db"`; update `load_example_config` test to create `dir.path()/anansi/` subdir and write the example config there before calling `Config::load(dir.path())`
- [x] `anansi.toml.example` -- update `[paths]` section to: `web_dir = "anansi/web"`, `rules_dir = "anansi/%Rules"`, `templates_dir = "anansi/templates"`, `db_file = "anansi/web.db"`
- [x] `src/vault.rs` -- rewrite `atomic_note_path`: remove the `-` prefix match; use `let ext = if entity_type.is_empty() { "note" } else { entity_type }; self.web.join(format!("{s}.{ext}.md"))`; rewrite `wikilink` to `format!("[[{s}.{ext}|{name}]]")` with same ext fallback; update `path_conventions` test expectations (e.g. person → `web/ian-kitajima.person.md`, organization → `web/pichtr.organization.md`, concept → `web/sovereign-ai.concept.md`, note → `web/some-note.note.md`); update `wikilink_formats` test expectations (e.g. person → `[[ian-kitajima.person|Ian Kitajima]]`)
- [x] `src/main.rs` -- in `cmd_init`: change dirs slice to `["anansi", "anansi/web", "anansi/%Rules", "anansi/templates"]`; change template seed dest to `root.join("anansi").join("templates").join(name)`; change rules seed dest to `root.join("anansi").join("%Rules").join(name)`; change `toml_dest` to `root.join("anansi").join("anansi.toml")`; `Config::load(root)` and `config.db_path(root)` calls are unchanged
- [x] `plugin/anansi.plugin/skills/anansi-remember/SKILL.md` -- write full 8-step skill (see Design Notes for step outline, entity taxonomy, TOC format grammar, and MCP setup note)
- [x] `plugin/anansi.plugin/commands/remember.md` -- /remember slash command invoking the anansi-remember skill; accepts optional vault root override
- [x] `plugin/anansi.plugin/.claude-plugin/plugin.json` -- update `name` to `"anansi"`, update `description` to cover both toc and remember skills, add `"remember"` and `"mcp-ingest"` to keywords array
- [x] `Dockerfile` -- change `VOLUME ["/anansi"]` to `VOLUME ["/vault"]`; change `ENV ANANSI_ROOT=/anansi` to `ENV ANANSI_ROOT=/vault`; change `CMD ["serve", "--root", "/anansi"]` to `CMD ["serve", "--root", "/vault"]`; update `HEALTHCHECK` curl URL if it referenced a path
- [x] `docker-compose.yml` -- update volume bind to `./vault:/vault`; update any env var defaults that referenced the old `/anansi` mount path

**Acceptance Criteria:**
- Given a fresh empty directory, when `anansi2 --root <dir> init`, then `<dir>/anansi/anansi.toml`, `<dir>/anansi/web.db`, `<dir>/anansi/web/`, `<dir>/anansi/templates/`, `<dir>/anansi/%Rules/` all exist; `<dir>/anansi.toml` and `<dir>/web.db` do NOT exist at the vault root level
- Given a vault with the new layout, when `anansi2 --root <dir> serve`, then the MCP server starts and `GET /health` returns 200
- Given a source file at `<dir>/my-notes.md` and a running MCP server, when `anansi_ingest` is called with `source_path: "<dir>/my-notes.md"`, then the resulting outline and leaf files appear in `<dir>/anansi/web/`
- Given `entity_type="person"` and `name="Ian Kitajima"`, when `atomic_note_path` is called, then the result path ends in `ian-kitajima.person.md`
- Given `entity_type=""`, when `atomic_note_path` is called, then the result path ends in `.note.md`
- Given `cargo check`, then exits 0 with no errors
- Given `cargo test --lib`, then all unit tests pass including updated vault `path_conventions` and `wikilink_formats` tests

## Design Notes

**Config path chain:** `Config::load(vault_root)` finds `vault_root/anansi/anansi.toml`. Default paths `"anansi/web"` etc. are joined against `vault_root` by the unchanged path helpers. Result: `vault_root/anansi/web`, `vault_root/anansi/web.db`, etc. No call sites change.

**`atomic_note_path` before/after:**
```
Before: ("person", "Ian Kitajima") → /vault/web/-ian-kitajima.md
After:  ("person", "Ian Kitajima") → /vault/anansi/web/ian-kitajima.person.md

Before: ("note", "some note") → /vault/web/some-note.md
After:  ("note", "some note") → /vault/anansi/web/some-note.note.md
```

**`wikilink` before/after:**
```
Before: ("person", "Ian Kitajima") → [[-ian-kitajima|Ian Kitajima]]
After:  ("person", "Ian Kitajima") → [[ian-kitajima.person|Ian Kitajima]]
```

**`anansi_toc` format (unchanged — entity_type already present):**
```
1 Digital Futures Workshop [topic] | summary:Strategic planning session...
1.1 James Pakele [person] | summary:Founder of Pakele.ai...
1.2 Anthropic [organization] | summary:AI safety company focused on alignment...
```

**`anansi-remember` SKILL.md step outline:**
1. **Identify context** — ask user for vault root path if not in conversation; confirm source filename.
2. **Read source** — get full source content from conversation, clipboard paste, or Read tool if path provided.
3. **Identify entities** — scan the full document; classify each meaningful entity using the taxonomy below. Use address notation for hierarchy (`1`, `1.1`, `1.1.1` etc.). Max depth 6.
4. **Entity taxonomy:**
   - `person` — named individual human
   - `organization` — company, institution, team, government body
   - `topic` — concept, technology, theme, discipline (not a person or org)
   - `concept` — abstract idea, framework, methodology
   - `note` — any entity that doesn't fit the above; default fallback
5. **Build anansi_toc block** — one leaf per line, format: `{address} {name} [{entity_type}] | summary:{one-sentence description}`. Include all leaves (not just top-level). The `?` entity_type triggers Pass 1 LLM fallback — do not use it; assign a real type.
6. **Splice frontmatter** — if source has existing YAML frontmatter (`---` block), insert `anansi_toc: |` and the TOC lines into it; if no frontmatter, prepend a new frontmatter block. All in memory — do not write to disk yet.
7. **Call anansi_ingest** — call the `anansi_ingest` MCP tool with `content: <augmented source string>` and `filename: <original filename>`. Report the returned `source_id` and `outline_note_id` on success.
8. **Fallback** — if the MCP tool is unavailable or returns an error, write the augmented content to `{vault_root}/{filename}` using the Write tool, then print: `anansi2 --root {vault_root} ingest {vault_root}/{filename}`.

**MCP setup note for SKILL.md:** The anansi MCP server must be running (`anansi2 --root <vault_root> serve`) and registered as an MCP server. Claude Desktop connects directly to `localhost:3738`. ChatGPT Developer Mode and Codex Desktop require an HTTPS tunnel (e.g. Cloudflare Tunnel forwarding to `localhost:3738`).

## Spec Change Log

## Verification

**Commands:**
- `cargo check` -- expected: exits 0, no errors
- `cargo test --lib` -- expected: all unit tests pass

## Suggested Review Order

**Config path resolution — entry point**

- Single line that drives the entire vault restructure; callers unchanged
  [`config.rs:94`](../../src/config.rs#L94)

- Four defaults that propagate the new layout to all consumers
  [`config.rs:25`](../../src/config.rs#L25)

**Typed filenames — leaf notes**

- `atomic_note_path` drops `-` prefix; uses `{slug}.{entity_type}.md` convention
  [`vault.rs:38`](../../src/vault.rs#L38)

- `wikilink` format matches new typed filenames for Obsidian resolution
  [`vault.rs:50`](../../src/vault.rs#L50)

**Init command — vault bootstrap**

- Dirs changed from flat to `anansi/` subdir hierarchy
  [`main.rs:95`](../../src/main.rs#L95)

- Template and rules seed destinations moved to `anansi/` subdir
  [`main.rs:122`](../../src/main.rs#L122)

- Config seeded to `anansi/anansi.toml` (was vault root)
  [`main.rs:145`](../../src/main.rs#L145)

**Plugin — anansi-remember skill**

- TOC format grammar with `hint:` annotation that flows into Pass 3
  [`SKILL.md:33`](../../plugin/anansi.plugin/skills/anansi-remember/SKILL.md#L33)

- Single `anansi_ingest content+filename` call — the "one motion" pattern
  [`SKILL.md:110`](../../plugin/anansi.plugin/skills/anansi-remember/SKILL.md#L110)

- Graceful fallback: write to disk + print CLI command when server unreachable
  [`SKILL.md:137`](../../plugin/anansi.plugin/skills/anansi-remember/SKILL.md#L137)

**Deployment**

- VOLUME and CMD renamed from `/anansi` to `/vault`
  [`Dockerfile:27`](../../Dockerfile#L27)

- Volume bind updated; existing deployments must migrate `./anansi` → `./vault`
  [`docker-compose.yml:7`](../../docker-compose.yml#L7)

**Tests and config**

- Vault unit tests updated to new `{slug}.{type}.md` expectations
  [`vault.rs:65`](../../src/vault.rs#L65)

- Config test creates `anansi/` subdir before loading
  [`config.rs:137`](../../src/config.rs#L137)

**Manual checks:**
- After `anansi2 --root /tmp/test-v4 init`: verify `/tmp/test-v4/anansi/anansi.toml` exists; verify `/tmp/test-v4/anansi.toml` does NOT exist
- After ingest of a source with person/organization/topic entities: verify leaf files in `anansi/web/` end in `.person.md`, `.organization.md`, `.topic.md` respectively

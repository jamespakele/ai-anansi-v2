---
title: 'Anansi v2 — Build 01: Foundation & Data Model'
type: 'feature'
created: '2026-04-23'
status: 'done'
baseline_commit: 'NO_VCS'
context:
  - docs/anansi-v2-spec.md
  - docs/anansi-v2-build-01-foundation.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The anansi v2 Rust crate does not exist. Without the structural skeleton — schema, config, template parser, rules loader, vault helpers, and seed content files — neither the pipeline (build-02) nor the interfaces (build-03) can be built.

**Approach:** Create the complete foundation per `docs/anansi-v2-build-01-foundation.md` in the order specified: Cargo.toml → migrations → config → db → templates (15 files) → template.rs → %Rules (4 files) → rules.rs → vault.rs. All four acceptance-criteria tests must pass under `cargo test --lib`.

## Boundaries & Constraints

**Always:**
- Use exact dependency versions from build-01 §1. Use `chrono = { version = "0.4", features = ["serde"] }` and `tempfile = "3"` dev-dep.
- DB schema must be verbatim from spec §10 (four tables, all indices, all FK declarations). `PRAGMA foreign_keys = ON` goes in the connection options, not the migration.
- `match_key` normalization must exactly match the spec §12 Rust example.
- Template parser must handle YAML frontmatter + `%% field %%` blocks + body in that order.
- All 15 templates must declare `entity_type`, `atomic`, `merge_strategy: "2.0"`, `description`, `identity_fields`, `sources`. Container types add `roster_sections`.
- `atomic_types()` must return 12 types (11 from spec §5 table + `outline`); `source_types()` must return 4.
- Vault path rules: pure-atomic/container → `-{slug}.md`; note type → `{slug}.md`; source-bound → `{addr}-{slug}-{source_slug}.md`; outline → `{source_slug}.outline.md`.

**Ask First:**
- Any dependency version change from what build-01 specifies.
- Adding modules beyond those in build-01 scope (e.g. any pipeline, LLM, MCP code).

**Never:**
- Implement `llm.rs`, `prompt.rs`, `pipeline.rs`, `writer.rs`, `merger.rs`, `mcp.rs`, `main.rs` — those are build-02/03.
- Add FTS5 tables or `field_conflicts` table (v1.5).
- Add Dockerfile or Cowork plugin files.
- Use `slug::slugify` in `match_key` — the spec gives an explicit hand-rolled normalization; use it.

## I/O & Edge-Case Matrix

| Scenario | Input | Expected Output | Error Handling |
|----------|-------|-----------------|----------------|
| match_key normal | `("Ian Kitajima", "person")` | `"person:ian-kitajima"` | — |
| match_key org acronym | `("PICHTR", "organization")` | `"organization:pichtr"` | — |
| match_key multi-word concept | `("Sovereign AI", "concept")` | `"concept:sovereign-ai"` | — |
| Template load all 15 | `templates/` dir with 15 `.md` files | All parse without error; registry has 12 atomic + 4 source types | `anyhow::Error` on parse failure |
| RuleRegistry load | `%Rules/` with 4 `%*.md` files | `get("Atomicity")` returns non-empty string | `anyhow::Error` if dir missing |
| Vault atomic path | `entity_type="person", name="Ian Kitajima"` | `web/-ian-kitajima.md` | — |
| Vault note path (exception) | `entity_type="note", name="some note"` | `web/some-note.md` (no `-` prefix) | — |
| Vault source-bound path | `toc_address="3.2", name="Sovereign AI discussion", source_slug="dfw-2026-01"` | `web/3-2-sovereign-ai-discussion-dfw-2026-01.md` | — |
| Vault outline path | `source_slug="digital-futures-workshop-2026-01-30"` | `web/digital-futures-workshop-2026-01-30.outline.md` | — |

</frozen-after-approval>

## Code Map

All files are new — project directory is empty.

- `Cargo.toml` -- crate manifest + exact deps from build-01
- `migrations/0001_schema.sql` -- four-table schema verbatim from spec §10
- `anansi.toml.example` -- example config template per build-01 §3
- `src/lib.rs` -- module declarations only (no logic)
- `src/config.rs` -- TOML loader + env-var overrides + path helpers (~120 lines)
- `src/db.rs` -- SqlitePool, match_key, typed records (SourceRecord/NoteRecord/SourceContributionRecord/EdgeRecord), CRUD fns (~350 lines)
- `templates/*.md` -- 15 entity type template files (person, concept, topic, area, note, organization, project, context, event, task, action_item_list, outline, container, email_thread, meeting_summary, research_paper)
- `src/template.rs` -- TemplateRegistry + Template + FieldBlock + RosterSection parser (~230 lines)
- `%Rules/%Atomicity.md` -- Rule 1 (reusability) + Rule 2 (entity purity) per build-01 §8
- `%Rules/%Downstream-Flow.md` -- context_at routing guidance (~80 lines)
- `%Rules/%Merge-Strategy.md` -- three merge categories per spec §12
- `%Rules/%Template-Schema.md` -- template authoring reference per spec §6
- `src/rules.rs` -- RuleRegistry: load `%*.md`, keyed by stem (~70 lines)
- `src/vault.rs` -- Vault struct: source/outline/atomic/source-bound paths + wikilink renderer (~140 lines)

## Tasks & Acceptance

**Execution:**
- [x] `Cargo.toml` -- create with exact deps from build-01 §1 including `chrono` and `tempfile` dev-dep
- [x] `migrations/0001_schema.sql` -- create verbatim 4-table schema from spec §10
- [x] `anansi.toml.example` -- create example config per build-01 §3
- [x] `src/lib.rs` -- declare all modules (config, db, template, rules, vault)
- [x] `src/config.rs` -- implement Config/PathsConfig/LlmConfig/InferSettings/ServerConfig with serde Deserialize, env-var overrides for ANANSI_OLLAMA_URL/MODEL/MCP_PORT, and web/rules/templates/db path helpers
- [x] `src/db.rs` -- implement DbPool type alias, open_and_migrate (WAL + FK pragma + sqlx::migrate!), match_key fn, four typed records with sqlx::FromRow, and all CRUD fns listed in build-01 §5; add unit tests for match_key covering the three spec §12 examples
- [x] `templates/concept.md` through `templates/research_paper.md` -- create all 15 template files with correct frontmatter, %% field blocks, and body templates; use person.md and organization.md from spec §6 verbatim as anchors
- [x] `src/template.rs` -- implement TemplateRegistry::load, Template struct, MergeStrategy enum, FieldBlock/RosterSection/SourceHint structs, %% block parser, body renderer; add unit tests: loads_all_fifteen_templates, atomic_types_returns_expected_set (12 types), source_types check (4 types)
- [x] `%Rules/%Atomicity.md` -- create per build-01 §8 verbatim
- [x] `%Rules/%Downstream-Flow.md` -- create per build-01 §9
- [x] `%Rules/%Merge-Strategy.md` -- create per build-01 §10
- [x] `%Rules/%Template-Schema.md` -- create per build-01 §11
- [x] `src/rules.rs` -- implement RuleRegistry::load (reads `%*.md` files, keys by stem sans `%`) and get; add test that all 4 canonical rules return non-empty body
- [x] `src/vault.rs` -- implement Vault::new, source_path, outline_path, atomic_note_path (with note exception), source_bound_path (`.` → `-` in address), wikilink; add unit tests covering all path conventions and wikilink format

**Acceptance Criteria:**
- Given the project root, when `cargo check` is run, then it exits 0 with no errors (warnings about unused items are acceptable at this stage)
- Given a fresh SQLite file, when `sqlite3 /tmp/test-anansi.db < migrations/0001_schema.sql` is run, then all four tables and their indices are created and `PRAGMA foreign_key_check` returns empty
- Given the templates directory with all 15 files, when `cargo test --lib db::tests::match_key_canonical_forms` is run, then it passes
- Given the templates directory with all 15 files, when `cargo test --lib template::tests::loads_all_fifteen_templates` is run, then it passes
- Given the templates directory, when `cargo test --lib template::tests::atomic_types_returns_expected_set` is run, then it passes (returns exactly 12 atomic types)
- Given the %Rules directory with all 4 files, when all tests under `vault::tests` and `rules` are run, then they pass

## Design Notes

**Template %% block parsing:** Split the file on `---` to extract frontmatter (first block), then scan the remainder for `%%...%%` delimiters. Each `%%` block is a mini-YAML payload with `field`, `description`, and optional `format`/`constraints` keys. Everything after the last `%%` closing delimiter is the body template.

**match_key vs slug:** These are two distinct functions. `match_key` is used for deduplication (normalise → hyphenate → prefix with entity_type). `slug::slugify` from the crate is used for filename generation. Do not conflate them.

**src/lib.rs:** Declare modules as `pub mod config; pub mod db; pub mod template; pub mod rules; pub mod vault;`. No main.rs yet — the binary entry point comes in build-03.

## Verification

**Commands:**
- `cargo check` -- expected: exits 0, no errors
- `sqlite3 /tmp/test-anansi.db < migrations/0001_schema.sql && sqlite3 /tmp/test-anansi.db "PRAGMA foreign_key_check;"` -- expected: no output (empty = clean)
- `cargo test --lib` -- expected: all tests pass (match_key_canonical_forms, loads_all_fifteen_templates, atomic_types_returns_expected_set, path_conventions)

## Spec Change Log

## Suggested Review Order

**Schema & Data Model**

- Verbatim spec §10 schema: 4 tables, 7 indices, all FKs with WAL+FK pragmas
  [`0001_schema.sql:1`](../../migrations/0001_schema.sql#L1)

- `match_key()`: spec §12 exact normalization — the dedup key for all note identity lookups
  [`db.rs:22`](../../src/db.rs#L22)

- `open_and_migrate()`: pool setup using `.filename()` (safe path handling) + embedded migrations
  [`db.rs:9`](../../src/db.rs#L9)

- `FromRow`-derived record structs matching every column in the schema
  [`db.rs:42`](../../src/db.rs#L42)

**Template System**

- `MergeStrategy` enum: three categories that drive all merge, write, and dedup decisions
  [`template.rs:14`](../../src/template.rs#L14)

- `render_body()`: single-pass regex substitution — prevents template injection from field values
  [`template.rs:87`](../../src/template.rs#L87)

- `parse_template()`: frontmatter → `%% field %%` blocks → body template pipeline
  [`template.rs:143`](../../src/template.rs#L143)

- `TemplateRegistry::load()`: reads all `.md` files; atomic/source-type split used throughout
  [`template.rs:99`](../../src/template.rs#L99)

- Example atomic (pure) template: person.md — identity fields only, no source-specific sections
  [`person.md:1`](../../templates/person.md#L1)

- Example container template: organization.md — roster_sections schema + set-union merge contract
  [`organization.md:1`](../../templates/organization.md#L1)

**Vault Paths**

- `slug_name()`: spec §13 exact algorithm, consistent with `match_key` normalization
  [`vault.rs:5`](../../src/vault.rs#L5)

- `Vault` path helpers: note prefix rules (`-`, none, `{addr}-`), outline `.outline.md` suffix
  [`vault.rs:33`](../../src/vault.rs#L33)

- `wikilink()`: Obsidian `[[target|display]]` format with correct per-type prefix
  [`vault.rs:52`](../../src/vault.rs#L52)

**Rules System**

- `RuleRegistry::load()`: reads `%*.md` files, strips frontmatter, keyed by stem
  [`rules.rs:9`](../../src/rules.rs#L9)

- %Atomicity.md: Rule 1 (reusability) + Rule 2 (entity purity) — injected into Pass 1 and Pass 3 prompts
  [`%Atomicity.md:1`](../../%25Rules/%25Atomicity.md#L1)

**Configuration**

- `Config::load()`: TOML → struct with env-var overrides for Docker deployment
  [`config.rs:88`](../../src/config.rs#L88)

**Tests & Peripherals**

- Four AC test functions (match_key, template loading, atomic types, vault paths)
  [`db.rs:277`](../../src/db.rs#L277), [`template.rs:256`](../../src/template.rs#L256), [`vault.rs:70`](../../src/vault.rs#L70)

---
title: 'Anansi v2 — Build 07: Template Taxonomy'
type: 'feature'
created: '2026-04-26'
status: 'complete'
baseline_commit: 'b229eb6'
completed_audit: '2026-04-27'
context:
  - docs/anansi-v2-spec.md
  - docs/handoff-template-taxonomy.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** All templates sit in a flat namespace with no structural signal about what tier they belong to. Pass-1 receives every entity type regardless of the source being processed (a meeting ingest sees `youtube_chapter` as a valid type). There is no stopping rule preventing the LLM from decomposing a `topic_discussion` into sub-leaves.

**Approach:** (1) Rename 10 existing template files with `identity-` or source-family prefixes (filename only — `entity_type` field inside each file unchanged). (2) Add `template_class` frontmatter field to every template and `floor_prompt` to content-unit templates. (3) Create 5 new floor and source templates (meeting, research, youtube, email families). (4) Add a `source_family` field parsed from filename at load time so the registry can filter by source family in-memory. (5) Update `build_pass1` to show only relevant entity types and inject the floor stopping rule for the active source family. (6) Add a TOC validator that rejects sub-leaves of content-unit nodes.

## Boundaries & Constraints

**Always:**
- Template `entity_type` field inside each file is unchanged during rename — registry keying by entity_type is unaffected
- `TemplateRegistry::load(dir)` signature is unchanged — no call-site updates needed in `main.rs`
- Filter by source family happens in-memory after load, not at load time
- `cargo check` and `cargo test --lib` must pass after all changes; fix every test that breaks
- Existing entity types (`person`, `organization`, `context`, `event`, `task`, `action_item_list`, etc.) continue to be valid TOC leaf types in all modes

**Ask First:**
- Any change to `MergeStrategy` variants or `merger.rs` dispatch logic
- Any DB schema or migration change

**Never:**
- Change `src/merger.rs`, `src/db.rs`, `src/mcp.rs`, or `migrations/`
- Change `entity_type` values inside renamed template files
- Remove or break the existing `%% field %%` block parsing in `src/template.rs`
- Add `template_class` filtering to `TemplateRegistry::load` — filtering is in-memory only
- Change Pass-3 or Pass-4 prompt assembly — floor-type leaves go through the same Pass-3 call as any other source-bound leaf

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Meeting ingest, Pass-1 | `source_type = meeting_summary` | Entity type list includes `topic_discussion`; does NOT include `youtube_chapter` or `article_section`; floor prompt for `topic_discussion` appended | — |
| Youtube ingest, Pass-1 | `source_type = youtube_video` | Entity type list includes `youtube_chapter`; does NOT include `topic_discussion`; floor prompt for `youtube_chapter` appended | — |
| Unknown source type | `source_type = "container"` (fallback) | Entity type list shows only identity + utility types; no floor prompts appended | — |
| TOC has content-unit child | `1.1 Topic A [topic_discussion]` then `1.1.1 Sub-point [context]` | `validate_preprocessed_toc` returns `Err` with message naming the parent address and type | — |
| Valid content-unit TOC | All `topic_discussion` leaves are terminal | Validation passes, Pass-3 loop proceeds normally | — |
| Renamed template loaded | `identity-person.md` in templates dir | Registry entry keyed `"person"` — all call sites using `registry.get("person")` unchanged | — |
| `template_class` absent | Old template file without the field | Parse fails with clear YAML error naming the file | Surfaces at registry load time |

</frozen-after-approval>

## Code Map

- `templates/` -- 10 files to rename + 6 files to edit (add frontmatter) + 5 new files
- `%Rules/%Atomicity.md` -- append Rule 3
- `src/template.rs:8` -- `MergeStrategy` enum; no change
- `src/template.rs:61` -- `TemplateFrontmatter` struct; add `template_class` and `floor_prompt` fields
- `src/template.rs:79` -- `Template` struct; add `template_class: TemplateClass`, `floor_prompt: Option<String>`, `source_family: Option<String>`
- `src/template.rs:106` -- `TemplateRegistry`; add `floor_prompts_for_source` and `types_for_source` methods
- `src/template.rs:111` -- `TemplateRegistry::load`; parse filename prefix into `source_family` — signature unchanged
- `src/template.rs:273` -- `loads_all_fifteen_templates` test; rename + fix count to 21
- `src/template.rs:286` -- `atomic_types_returns_expected_set`; update count to 16
- `src/template.rs:309` -- `source_types_returns_four`; rename + update count to 5
- `src/prompt.rs:57` -- `build_pass1`; add `source_type: &str` param, use `types_for_source`, inject `{FLOOR_RULES}`
- `src/prompt.rs:144` -- `build_pass1_contains_rules_and_entity_types` test; add `source_type` arg
- `prompts/pass-1-toc-extraction.md` -- add `{FLOOR_RULES}` placeholder before `{SOURCE}`
- `src/pipeline.rs:269` -- `validate_preprocessed_toc`; add `validate_no_floor_children` check
- `src/pipeline.rs:548` -- `build_pass1` call (TOC fallback path); add `&source_type` arg
- `src/pipeline.rs:558` -- `build_pass1` call (standard path); add `&source_type` arg

## Tasks & Acceptance

**Execution:**

- [x] `templates/` -- rename 10 files using `git mv`: `person.md` → `identity-person.md`, `organization.md` → `identity-organization.md`, `concept.md` → `identity-concept.md`, `topic.md` → `identity-topic.md`, `area.md` → `identity-area.md`, `note.md` → `identity-note.md`, `project.md` → `identity-project.md`, `meeting_summary.md` → `meeting-summary.md`, `research_paper.md` → `research-paper.md`, `email_thread.md` → `email-thread.md`; do NOT change any file contents yet; run `cargo test --lib` — must still pass (registry keys by entity_type, not filename)

- [x] `templates/` -- add `template_class` to frontmatter of all existing files (no other changes): `identity-*` files → `template_class: identity`; `meeting-summary.md`, `research-paper.md`, `email-thread.md` → `template_class: source`; `context.md`, `event.md`, `task.md`, `action_item_list.md`, `outline.md`, `container.md` → `template_class: utility`

- [x] `templates/` -- create 5 new template files with full content from `docs/handoff-template-taxonomy.md §5`: `meeting-topic-discussion.md` (entity_type: topic_discussion, template_class: content_unit, atomic: true, floor_prompt present), `research-section.md` (entity_type: article_section, content_unit, atomic: true), `youtube-video.md` (entity_type: youtube_video, template_class: source, atomic: false), `youtube-chapter.md` (entity_type: youtube_chapter, content_unit, atomic: true), `email-exchange.md` (entity_type: email_exchange, content_unit, atomic: true)

- [x] `src/template.rs` -- add `TemplateClass` enum with variants `Identity`, `ContentUnit`, `Source`, `Utility` and `impl<'de> Deserialize<'de>` matching strings `"identity"`, `"content_unit"`, `"source"`, `"utility"`; add `#[derive(Debug, Clone, PartialEq)]`; add `template_class: TemplateClass` and `floor_prompt: Option<String>` to both `TemplateFrontmatter` and `Template`; add `source_family: Option<String>` to `Template` only (derived from filename at load time, not frontmatter); propagate all three new fields in `parse_template` return

- [x] `src/template.rs` -- in `TemplateRegistry::load`, after computing `template`, extract the filename stem and parse `source_family`: if stem starts with `"identity-"` or has no `-` prefix (utility types) → `None`; otherwise take the prefix up to the first `-` → `Some(prefix)`; set `template.source_family = source_family`; signature `load(dir: &Path)` is unchanged

- [x] `src/template.rs` -- add two new methods to `TemplateRegistry`: (1) `pub fn types_for_source(&self, source_type: &str) -> Vec<&str>` — returns entity types for `Identity` class + `Utility` class + any type whose `source_family` matches `source_family_of(source_type)`, sorted; (2) `pub fn floor_prompts_for_source(&self, source_type: &str) -> Vec<&str>` — returns `floor_prompt` strings from `ContentUnit` templates whose `source_family` matches `source_family_of(source_type)`; add private `fn source_family_of(source_type: &str) -> &str` mapping `"meeting_summary"→"meeting"`, `"research_paper"→"research"`, `"email_thread"→"email"`, `"youtube_video"→"youtube"`, `_→""`

- [x] `src/template.rs` -- fix broken tests: rename `loads_all_fifteen_templates` to `loads_all_templates`, update assertion to `21` (16 original + 5 new); update `atomic_types_returns_expected_set` count to `16` and add the 4 new atomic types to the expected set; rename `source_types_returns_four` to `source_types_returns_five`, update count to `5`

- [x] `src/prompt.rs` -- update `build_pass1` signature to `pub fn build_pass1(rules: &RuleRegistry, templates: &TemplateRegistry, source: &str, source_type: &str) -> Result<String>`; replace `render_entity_types(templates)` call with a version that calls `templates.types_for_source(source_type)`; after injecting `{ENTITY_TYPES}`, collect floor prompts via `templates.floor_prompts_for_source(source_type)`, join them with `\n`, and inject into a new `{FLOOR_RULES}` placeholder; fix the `build_pass1_contains_rules_and_entity_types` test to pass `"meeting_summary"` as `source_type`

- [x] `prompts/pass-1-toc-extraction.md` -- add `{FLOOR_RULES}` placeholder after the entity type list section and before the source body injection point; the placeholder resolves to empty string when no floor types apply (i.e., `build_pass1` already handles the empty case — the template just needs the placeholder present)

- [x] `src/pipeline.rs` -- add `fn validate_no_floor_children(leaves: &[TocLeaf], registry: &TemplateRegistry) -> Result<()>` that iterates leaves, checks if `registry.get(&leaf.entity_type).map(|t| t.template_class == TemplateClass::ContentUnit).unwrap_or(false)`, and if so verifies no other leaf's address starts with `format!("{}.", leaf.address)`; call this function inside `validate_preprocessed_toc` after existing address checks; update `validate_preprocessed_toc`'s import to include `TemplateClass`

- [x] `src/pipeline.rs` -- update both `prompt::build_pass1` call sites (lines 548 and 558) to pass `&source_type` as the new fourth argument; `source_type` is already bound at line 515 from the source frontmatter

- [x] `%Rules/%Atomicity.md` -- append Rule 3 — Minimum Viable Granularity as specified in `docs/handoff-template-taxonomy.md §6.4`

**Acceptance Criteria:**

- Given `cargo build`, then exits 0 after template renames (before any Rust changes)
- Given a `meeting_summary` ingest, when Pass-1 runs, then the prompt contains `topic_discussion` in the entity type list, does NOT contain `youtube_chapter`, and contains the `floor_prompt` text from `meeting-topic-discussion.md`
- Given a TOC where a `topic_discussion` leaf at address `1.1` has a child leaf at `1.1.1`, when `validate_preprocessed_toc` runs, then it returns `Err` with a message naming address `1.1` and type `topic_discussion`
- Given a valid TOC with only terminal `topic_discussion` leaves, when validation runs, then it passes and Pass-3 produces expanded notes
- Given `registry.get("person")`, then it returns the `identity-person.md` template with `entity_type == "person"` (rename did not break keying)
- Given `cargo test --lib`, then all tests pass with updated counts

## Design Notes

**Why `source_family` on `Template` instead of changing `load` signature:** `TemplateRegistry::load` is called from `main.rs:170`, `main.rs:213`, and multiple test helpers. Changing the signature would require updating every call site with a source type that isn't known until `ingest()` runs. Parsing the filename prefix once at load time and storing it on the struct lets all existing call sites work unchanged while enabling in-memory filtering.

**`types_for_source` replaces `atomic_types` in `build_pass1`:** After build-07, `atomic_types()` returns 16 types (including all floor types). Pass-1 for a meeting summary must not see `youtube_chapter` in its type list. `types_for_source("meeting_summary")` returns identity + utility + meeting-family types only.

**`{FLOOR_RULES}` injection placement:** The placeholder must be added to `pass-1-toc-extraction.md` BEFORE the Rust code change to `build_pass1` — otherwise the injection call will silently no-op (the placeholder isn't in the template, `inject` finds nothing to replace, no error). Do the template file edit first in the task sequence.

**`template_class` as required field:** Adding it as a required field (no `#[serde(default)]`) means any template file missing it will fail to parse at registry load time. This is intentional — the error appears immediately at startup, not silently at prompt assembly. The task sequence adds the field to all existing files before updating the Rust parser, so the repo is never in a state where load would fail.

**Floor template in Pass-3:** `topic_discussion`, `article_section`, `youtube_chapter`, `email_exchange` are `merge_strategy: source_bound`. The existing Pass-3 loop handles them identically to `context` or `event` — no pipeline changes needed.

## Implementation Audit (2026-04-27)

Audit discovered that all build-07 tasks were **already fully implemented** in the codebase.

| Component | Status | Notes |
|-----------|--------|-------|
| `templates/` — 10 renames | ✅ Complete | `identity-person.md`, `identity-organization.md`, `identity-concept.md`, `identity-topic.md`, `identity-area.md`, `identity-note.md`, `identity-project.md`, `meeting-summary.md`, `research-paper.md`, `email-thread.md` |
| `templates/` — `template_class` in all 21 files | ✅ Complete | No file missing the field |
| `templates/` — 5 new files | ✅ Complete | `meeting-topic-discussion.md`, `research-section.md`, `youtube-video.md`, `youtube-chapter.md`, `email-exchange.md` |
| `src/template.rs` — `TemplateClass` enum + new fields | ✅ Complete | `TemplateClass`, `template_class`, `floor_prompt`, `source_family` all present |
| `src/template.rs` — `source_family` parsing in `load()` | ✅ Complete | Filename-prefix logic present, signature unchanged |
| `src/template.rs` — `types_for_source` + `floor_prompts_for_source` | ✅ Complete | Both methods present with `source_family_of` helper |
| `src/template.rs` — updated tests | ✅ Complete | `loads_all_templates` (21), `atomic_types_returns_expected_set` (16), `source_types_returns_five` (5) |
| `src/prompt.rs` — `build_pass1` updated signature | ✅ Complete | 4-arg signature with `source_type`, uses `types_for_source` and `floor_prompts_for_source`, injects `{FLOOR_RULES}` |
| `prompts/pass-1-toc-extraction.md` — `{FLOOR_RULES}` placeholder | ✅ Complete | Present at line 55 |
| `src/pipeline.rs` — `validate_no_floor_children` | ✅ Complete | Implemented and called inside `validate_preprocessed_toc` |
| `src/pipeline.rs` — `build_pass1` call sites updated | ✅ Complete | Both call sites (lines 631 and 641) pass `&source_type` |
| `%Rules/%Atomicity.md` — Rule 3 appended | ✅ Complete | "Minimum Viable Granularity" at line 40 |

## Verification

**Automated (run 2026-04-27):**
- `cargo check` — ✅ exits 0, no warnings
- `cargo test --lib` — ✅ 36/36 tests pass (21 templates, 16 atomic, 5 source)

**Manual checks (pending — requires a live ingest):**
- Meeting summary ingest: confirm `topic_discussion` in Pass-1 prompt, `youtube_chapter` absent, floor prompt text present
- Preprocessed TOC with a `topic_discussion` child leaf: confirm `validate_preprocessed_toc` returns descriptive `Err`, not panic
- Confirm `registry.get("person")` still resolves after rename (entity_type key unchanged)

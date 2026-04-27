# Anansi v2 — Build Document 7: Template Taxonomy Redesign

**For:** Coding agent (Claude Code) executing against the repo produced by docs 1–6.
**Reference:** `anansi-v2-spec.md` §5 (template registry), §9 (TOC schema), §11 (pipeline Pass 1).
**Prerequisite:** Builds 01–06 complete. Pipeline, merger, db, and mcp are all working. Templates exist as flat files in `templates/` (person.md, meeting_summary.md, etc.). `cargo build` and all existing tests pass.
**Does NOT touch:** `src/merger.rs`, `src/db.rs`, `src/mcp.rs`, `migrations/`, `src/pipeline.rs` Pass 3 loop.
**Output:** A three-tier template namespace with `template_class` and `floor_prompt` frontmatter, selective template loading by source family, floor constraint validation in the TOC validator, and Pass 1 prompt injection of floor stopping rules.

---

## Goal

The current template system uses a flat namespace — `person.md`, `meeting_summary.md` — with no structural signal about what tier of the knowledge hierarchy a template belongs to. This causes two problems: all templates are loaded for every ingest regardless of relevance, and the Pass 1 prompt has no template-driven way to communicate decomposition stopping rules to the LLM.

This build introduces a naming convention that encodes template tier in the filename prefix, two new frontmatter fields (`template_class` and `floor_prompt`), four new content-unit floor templates, selective template loading by source family, a floor constraint in the TOC validator, and floor prompt injection into Pass 1.

**Implementation order — follow exactly to keep the codebase compiling at each step:**

1. Rename existing template files (`git mv`) — no code changes
2. Add `template_class` to renamed files' frontmatter — text edits only
3. Create new floor and source templates — new files
4. Update `TemplateRegistry` struct to hold `template_class` and `floor_prompt` fields
5. Update `parse_template` / template loader to parse the new frontmatter fields
6. Update `TemplateRegistry::load` to accept a `source_family` parameter and load selectively
7. Update `build_pass1` in `src/prompt.rs` to inject `floor_prompt` fields
8. Add `validate_no_floor_children` to `src/pipeline.rs` and call it from `validate_preprocessed_toc` and the Pass 1 parse path
9. Update `anansi/%Rules/%Atomicity.md` with Rule 3

---

## Step 1 — Rename existing template files

Run these `git mv` commands from the repo root. The `entity_type` field **inside** each file stays the same — only the filename changes.

```bash
git mv templates/person.md            templates/identity-person.md
git mv templates/organization.md      templates/identity-organization.md
git mv templates/concept.md           templates/identity-concept.md
git mv templates/topic.md             templates/identity-topic.md
git mv templates/area.md              templates/identity-area.md
git mv templates/note.md              templates/identity-note.md
git mv templates/project.md           templates/identity-project.md
git mv templates/meeting_summary.md   templates/meeting-summary.md
git mv templates/research_paper.md    templates/research-paper.md
git mv templates/email_thread.md      templates/email-thread.md
```

After renames, `cargo build` should still pass (template registry loads by glob, not by hardcoded name).

---

## Step 2 — Add `template_class` to renamed files' frontmatter

Edit the YAML frontmatter of each renamed file to add one line. Insert `template_class` immediately after `entity_type`. **Do not change any other field.**

### `identity-*` files — add `template_class: identity`

Apply to: `identity-person.md`, `identity-organization.md`, `identity-concept.md`, `identity-topic.md`, `identity-area.md`, `identity-note.md`, `identity-project.md`.

Example diff for `identity-person.md`:
```yaml
entity_type: person
template_class: identity     # ADD THIS LINE
atomic: true
```

### `meeting-summary.md`, `research-paper.md`, `email-thread.md` — add `template_class: source`

```yaml
entity_type: meeting_summary
template_class: source       # ADD THIS LINE
atomic: false
```

### Utility types — add `template_class: utility`

Apply to: `context.md`, `event.md`, `task.md`, `action_item_list.md`, `outline.md`, `container.md`.

```yaml
entity_type: context
template_class: utility      # ADD THIS LINE
atomic: true
```

---

## Step 3 — Create new templates

### `templates/meeting-topic-discussion.md`

```markdown
---
entity_type: topic_discussion
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A bounded discussion of a single topic within a meeting. Captures who
  participated, what was said, what was decided, and what actions followed.
  This is the atomization floor for meeting-summary sources — do not
  decompose further.
floor_prompt: |
  Decompose this meeting summary into topic_discussion leaves — one per
  distinct topic of discussion. Do not create sub-leaves within a topic
  discussion. Individual statements, speaker turns, and sub-points belong
  inside the leaf as content, not as separate TOC entries. If two agenda
  items blur together in the source, create one leaf. A side comment that
  launches a new direction is a new leaf only if it constitutes a distinct
  topic with its own participants and outcome.
sources:
  meeting_summary:
    hint: >
      Extract what was discussed, not a verbatim transcript. One sentence
      per substantive point. Capture decisions and action items precisely —
      these are the high-value outputs. Participants are people who spoke
      or were directly addressed, not everyone in the room.
---

%%
field: topic_sentence
description: One sentence naming the topic and framing what was being discussed
format: prose
constraints: "Source-specific — name the topic as it was framed in this meeting, not a generic definition"
%%

%%
field: participants
description: People who actively participated in this specific discussion
format: bullets
constraints: "Only people who spoke or were directly addressed in this topic, not all meeting attendees"
%%

%%
field: organizations
description: Organizations mentioned or represented in this discussion
format: bullets
%%

%%
field: content
description: Summary of what was discussed — the substance of the conversation
format: prose
constraints: "3-6 sentences. What was said, not who said it. No verbatim quotes."
%%

%%
field: decisions
description: Decisions reached or agreements made during this discussion
format: bullets
constraints: "Concrete outcomes only. If no decision was reached, write '[no decision reached]'"
%%

%%
field: action_items
description: Specific commitments and next steps from this discussion, with owners where known
format: bullets
constraints: "Format: 'Owner to do X by date' where information is available"
%%

# {{topic_sentence}}

## Participants
{{participants}}

## Organizations
{{organizations}}

## Content
{{content}}

## Decisions
{{decisions}}

## Action Items
{{action_items}}
```

### `templates/research-section.md`

```markdown
---
entity_type: article_section
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A subsection or paragraph of a research paper or article that makes one
  complete argument, claim, or point. This is the atomization floor for
  research-paper sources — do not decompose into individual sentences or
  claims.
floor_prompt: |
  Decompose this research paper into article_section leaves — one per
  distinct subsection or coherent argumentative unit. A leaf must be able
  to stand alone: a complete claim with its supporting evidence or
  reasoning. Do not create separate leaves for individual sentences,
  data points, or sub-claims within an argument. An introduction gets one
  leaf. A conclusion gets one leaf. Each numbered or headed subsection
  typically gets one leaf unless it contains two clearly independent
  arguments, in which case it may get two.
sources:
  research_paper:
    hint: >
      Capture the argument, not just the topic. The topic sentence is the
      claim being made. Supporting details are the evidence. Do not
      summarize so aggressively that the distinction between claim and
      evidence is lost.
---

%%
field: topic_sentence
description: The central claim or point this section makes
format: prose
constraints: "One sentence. This is the argument, not just the subject."
%%

%%
field: supporting_details
description: Evidence, examples, data, or reasoning that supports the topic sentence
format: bullets
constraints: "3-7 bullets. Concrete and specific. No repetition of the topic sentence."
%%

%%
field: content
description: Full prose summary of this section preserving the author's argument
format: prose
constraints: "2-4 sentences. Maintain the argumentative structure — do not flatten into a topic list."
%%

%%
field: entities
description: Concepts, people, organizations, and references mentioned in this section
format: bullets
%%

# {{topic_sentence}}

## Supporting Details
{{supporting_details}}

## Content
{{content}}

## Entities
{{entities}}
```

### `templates/youtube-video.md`

```markdown
---
entity_type: youtube_video
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: >
  A YouTube video or recorded presentation. Non-atomic source type —
  decomposes into youtube_chapter floor nodes.
---

%%
field: overview
description: >
  Convergence narrative — who made this video, why, what it is trying to
  convey, and what context it sits in. Who is the intended audience.
format: prose
%%

%%
field: topics
description: >
  Per-chapter breakdown using ### subsections, one per youtube_chapter
  leaf. Each subsection contains wikilinks to entities mentioned in
  that chapter.
format: prose
%%

# {{title}}

## Overview
{{overview}}

## Topics
{{topics}}
```

### `templates/youtube-chapter.md`

```markdown
---
entity_type: youtube_chapter
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A chapter or inferred thematic segment of a YouTube video or recorded
  presentation. This is the atomization floor for youtube-video sources —
  do not decompose into individual statements or timestamps.
floor_prompt: |
  Decompose this video transcript into youtube_chapter leaves — one per
  explicit chapter marker or inferred thematic segment. A segment ends
  when the speaker clearly transitions to a new topic or the content
  shifts focus. Do not create leaves for individual statements, examples,
  or asides within a segment. A brief tangent that returns to the main
  thread is content inside the current leaf, not a new leaf. Aim for
  5-15 leaves for a standard one-hour video — fewer for tightly focused
  content, more for wide-ranging discussions.
sources:
  youtube_video:
    hint: >
      Capture the substance of what was presented in this segment, not a
      transcript summary. What was the key point? What examples or
      evidence were given? What entities were mentioned?
---

%%
field: topic_sentence
description: One sentence describing what this chapter or segment is about
format: prose
%%

%%
field: key_points
description: The main points, arguments, or demonstrations in this segment
format: bullets
constraints: "3-7 bullets. Concrete and specific."
%%

%%
field: content
description: Prose summary of what was covered in this segment
format: prose
constraints: "2-4 sentences."
%%

%%
field: entities
description: People, organizations, concepts, and tools mentioned in this segment
format: bullets
%%

# {{topic_sentence}}

## Key Points
{{key_points}}

## Content
{{content}}

## Entities
{{entities}}
```

### `templates/email-exchange.md`

```markdown
---
entity_type: email_exchange
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A cohesive exchange within an email thread organized around one question,
  decision, or topic. This is the atomization floor for email-thread
  sources.
floor_prompt: |
  Decompose this email thread into email_exchange leaves — one per
  coherent exchange around a distinct question, decision, or topic. An
  exchange ends when the thread moves to a new subject or a decision
  is reached. Do not create separate leaves for individual emails within
  an exchange. A back-and-forth negotiation about one decision is one
  leaf. A thread that pivots to a new topic partway through gets a new
  leaf at the pivot point.
sources:
  email_thread:
    hint: >
      Capture what was being resolved, not who sent what when. The
      participants are who was in the exchange. The outcome is what
      was decided or agreed.
---

%%
field: subject
description: What question or topic this exchange is about
format: prose
%%

%%
field: participants
description: People involved in this specific exchange
format: bullets
%%

%%
field: content
description: Summary of the exchange — what was being discussed and how it unfolded
format: prose
%%

%%
field: outcome
description: The decision, agreement, or resolution reached in this exchange
format: prose
constraints: "If unresolved, write '[unresolved — thread continues]'"
%%

# {{subject}}

## Participants
{{participants}}

## Content
{{content}}

## Outcome
{{outcome}}
```

---

## Step 4 — Update `src/template.rs`

### 4.1 Add `TemplateClass` enum

Add this enum after the `MergeStrategy` enum (before `FieldDef`):

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TemplateClass {
    #[default]
    Utility,
    Identity,
    ContentUnit,
    Source,
}

impl<'de> Deserialize<'de> for TemplateClass {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "identity"     => Ok(TemplateClass::Identity),
            "content_unit" => Ok(TemplateClass::ContentUnit),
            "source"       => Ok(TemplateClass::Source),
            "utility"      => Ok(TemplateClass::Utility),
            other => Err(serde::de::Error::custom(format!("unknown template_class: {other}"))),
        }
    }
}
```

### 4.2 Add fields to `TemplateFrontmatter`

In the private `TemplateFrontmatter` struct, add two new fields:

```rust
#[derive(Debug, Clone, Deserialize)]
struct TemplateFrontmatter {
    entity_type: String,
    #[serde(default)]
    template_class: TemplateClass,   // ADD
    #[serde(default)]
    floor_prompt: Option<String>,    // ADD
    // ... existing fields unchanged ...
    #[serde(default)]
    atomic: bool,
    merge_strategy: MergeStrategy,
    // ...
}
```

### 4.3 Add fields to the public `Template` struct

```rust
#[derive(Debug, Clone)]
pub struct Template {
    pub entity_type: String,
    pub template_class: TemplateClass,   // ADD
    pub floor_prompt: Option<String>,    // ADD
    pub atomic: bool,
    pub merge_strategy: MergeStrategy,
    // ... existing fields unchanged ...
}
```

### 4.4 Propagate in `parse_template`

In `parse_template`, add the two new fields to the `Ok(Template { ... })` constructor:

```rust
Ok(Template {
    entity_type: fm.entity_type,
    template_class: fm.template_class,   // ADD
    floor_prompt: fm.floor_prompt,       // ADD
    atomic: fm.atomic,
    merge_strategy: fm.merge_strategy,
    // ... existing fields unchanged ...
})
```

### 4.5 Update `TemplateRegistry::load` to accept `source_family`

Replace the current `load` signature and glob logic:

```rust
impl TemplateRegistry {
    /// Load templates for the given source family.
    ///
    /// Always loads:
    /// - All `identity-*.md` templates
    /// - Utility types by explicit name: context, event, task, action_item_list, outline, container
    ///
    /// Conditionally loads:
    /// - All `{source_family}-*.md` templates (source type + its floor types)
    ///
    /// Pass `None` for source_family to load identity + utility only (used in tests).
    pub fn load(templates_dir: &Path, source_family: Option<&str>) -> Result<Self> {
        let mut templates = HashMap::new();

        let family = source_family.unwrap_or("container");
        let family_glob = format!("{}-*.md", family);

        // Explicit utility filenames — always loaded
        let utility_names: &[&str] = &[
            "context.md",
            "event.md",
            "task.md",
            "action_item_list.md",
            "outline.md",
            "container.md",
        ];

        let entries = std::fs::read_dir(templates_dir)
            .with_context(|| format!("reading templates dir: {}", templates_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let should_load = filename.starts_with("identity-")
                || utility_names.contains(&filename)
                || (!family_glob.starts_with("container-") && filename.starts_with(&format!("{}-", family)));

            if !should_load {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading template: {}", path.display()))?;

            let template = parse_template(&content)
                .with_context(|| format!("parsing template: {}", path.display()))?;

            templates.insert(template.entity_type.clone(), template);
        }

        Ok(Self { templates })
    }
    // ... rest of impl unchanged ...
}
```

**Note on `container` fallback:** When `source_family` is `"container"` (the default for unknown source types), the condition `!family_glob.starts_with("container-")` is `false`, so no source-family templates are loaded beyond `container.md` from the utility list. This is intentional.

### 4.6 Add `source_family` helper function to `TemplateRegistry`

Add as a `pub fn` inside `impl TemplateRegistry`:

```rust
/// Map a source_type string to its template family prefix.
pub fn source_family(source_type: &str) -> &'static str {
    match source_type {
        "meeting_summary" => "meeting",
        "research_paper"  => "research",
        "email_thread"    => "email",
        "youtube_video"   => "youtube",
        _                 => "container",
    }
}
```

### 4.7 Update test `loads_all_fifteen_templates`

The existing test calls `TemplateRegistry::load(&dir)` — update it to pass `Some("meeting")` (or any family) so all template families are loaded for count validation. Adjust the expected count to reflect the new templates.

The new total: 7 identity + 6 utility + 1 meeting-summary (source) + 1 meeting-topic-discussion (content_unit) + 1 research-paper + 1 research-section + 1 youtube-video + 1 youtube-chapter + 1 email-thread + 1 email-exchange = **21 templates** when all families are loaded.

Update the test to load all families for the count check by loading each family and merging, or simplify to load a known family and assert the count for that load. The simplest approach: load with `source_family = None` and assert the identity + utility count (13), then separately verify a meeting family load contains `topic_discussion`.

The test name `loads_all_fifteen_templates` should be renamed to `loads_identity_and_utility_templates` to match the new behavior.

---

## Step 5 — Update `src/prompt.rs` — Pass 1 floor prompt injection

In `build_pass1`, after the entity types injection and before `{SOURCE}` injection, collect and inject floor prompts from all loaded `content_unit` templates:

```rust
pub fn build_pass1(
    rules: &RuleRegistry,
    templates: &TemplateRegistry,
    source: &str,
) -> Result<String> {
    let mut prompt = PASS1_TEMPLATE.to_string();
    prompt = inject_rule(&prompt, "Atomicity", rules)?;
    prompt = inject_rule(&prompt, "Downstream-Flow", rules)?;

    let entity_types = render_entity_types(templates);
    prompt = inject(&prompt, "ENTITY_TYPES", &entity_types);

    // Collect floor prompts from all content_unit templates in the loaded registry
    let floor_prompts: Vec<String> = {
        let mut fps: Vec<String> = templates
            .templates()                           // add this accessor (see below)
            .filter(|t| t.template_class == TemplateClass::ContentUnit)
            .filter_map(|t| t.floor_prompt.clone())
            .collect();
        fps.sort(); // deterministic order
        fps
    };

    if !floor_prompts.is_empty() {
        let floor_section = format!(
            "\n\n## Decomposition Floor Rules\n\n{}",
            floor_prompts.join("\n")
        );
        prompt = inject(&prompt, "FLOOR_RULES", &floor_section);
    } else {
        prompt = inject(&prompt, "FLOOR_RULES", "");
    }

    prompt = inject(&prompt, "SOURCE", source);
    Ok(prompt)
}
```

Add the `{FLOOR_RULES}` placeholder to `prompts/pass-1-toc-extraction.md` — insert it after the `{ENTITY_TYPES}` section and before the source body injection point. The exact placement: after the entity type list ends and before `## Source`.

Also add the `templates()` iterator accessor to `TemplateRegistry` in `src/template.rs`:

```rust
pub fn templates(&self) -> impl Iterator<Item = &Template> {
    self.templates.values()
}
```

Import `TemplateClass` in `src/prompt.rs`:
```rust
use crate::template::{TemplateClass, TemplateRegistry};
```

---

## Step 6 — Update `src/pipeline.rs` — TOC floor constraint validator

### 6.1 Add `validate_no_floor_children`

Add this function after `validate_preprocessed_toc`:

```rust
/// Validate that no TOC leaf has a child that is a content_unit floor type.
/// A content_unit leaf (e.g., topic_discussion) cannot have children because
/// it is the decomposition floor for its source family.
pub fn validate_no_floor_children(
    leaves: &[TocLeaf],
    registry: &TemplateRegistry,
) -> Result<()> {
    use crate::template::TemplateClass;

    for leaf in leaves {
        let is_floor = registry
            .get(&leaf.entity_type)
            .map(|t| t.template_class == TemplateClass::ContentUnit)
            .unwrap_or(false);

        if is_floor {
            let child_prefix = format!("{}.", leaf.address);
            for other in leaves {
                if other.address.starts_with(&child_prefix) {
                    return Err(anyhow!(
                        "TOC validation error: leaf {} ({}) is a content_unit floor type \
                         and cannot have children. Found child at address {} ({}).",
                        leaf.address,
                        leaf.entity_type,
                        other.address,
                        other.entity_type
                    ));
                }
            }
        }
    }
    Ok(())
}
```

### 6.2 Call it from `validate_preprocessed_toc`

In `validate_preprocessed_toc`, after the existing duplicate-address and depth checks, add:

```rust
    // Floor constraint: content_unit leaves must have no children
    validate_no_floor_children(&leaves, templates)?;

    Ok(leaves)
```

### 6.3 Call it from the main `ingest` pipeline

In the `ingest` function, after `parse_toc` returns `leaves` (step 6 in `ingest`), add the floor validation:

```rust
    // 6. Parse TOC
    let leaves = parse_toc(&toc_text);

    // Validate floor constraints (content_unit leaves cannot have children)
    validate_no_floor_children(&leaves, &ctx.templates)?;
```

### 6.4 Update `IngestContext` to pass source family to template loader

`IngestContext` stores `templates: TemplateRegistry`. The templates must now be loaded with the correct source family for the source being ingested. Update the call sites in `src/main.rs` and `src/mcp.rs` that construct `IngestContext` to load templates with the source family:

```rust
// When constructing IngestContext, determine source_type first, then load templates:
let source_type = /* read from source file frontmatter or config */;
let family = TemplateRegistry::source_family(&source_type);
let templates = TemplateRegistry::load(&templates_dir, Some(family))?;
```

Search for `TemplateRegistry::load` calls across the codebase and update each one to pass the source family. The function signature change will produce compile errors at every call site — use those as a guide.

---

## Step 7 — Update `prompts/pass-1-toc-extraction.md`

Add the `{FLOOR_RULES}` placeholder to the Pass 1 prompt template. Insert it between the entity type list section and the source body section. The exact insertion point: find the line that injects `{ENTITY_TYPES}` and add `{FLOOR_RULES}` on a new line immediately after the entity type block ends, before `## Source` (or whatever heading precedes `{SOURCE}`).

The floor rules section will be empty string when no content_unit templates are loaded (container/fallback case), so no structural changes to the prompt are visible in that case.

---

## Step 8 — Update `anansi/%Rules/%Atomicity.md`

Append Rule 3 to the existing file:

```markdown
## Rule 3 — Minimum Viable Granularity

Do not decompose below the natural content unit for the source type. A
topic discussion, article section, video chapter, or email exchange is
the floor. A sentence is not a note. A sub-point within an argument is
not a note.

The test: can this unit be read without the surrounding source and still
convey complete, standalone meaning? If yes, it may be a content unit.
If it requires surrounding context to make sense, it belongs inside the
content unit above it.

Floor types are declared in the template file for each source family.
The `floor_prompt` field in each content_unit template encodes the
specific stopping rule for that source-floor pair.
```

---

## What NOT to change

- `src/merger.rs` — merge strategies are unchanged; `topic_discussion`, `article_section`, `youtube_chapter`, `email_exchange` all use `merge_strategy: source_bound`, which is already implemented
- `src/db.rs` — schema unchanged; new entity types produce rows in the same `notes` table
- `src/mcp.rs` — no interface changes (only update the `TemplateRegistry::load` call site to pass source family)
- `migrations/` — no schema changes
- `src/pipeline.rs` Pass 3 loop — each floor-type leaf goes through the same Pass 3 call as any other source-bound leaf; the loop body is unchanged
- Existing `context`, `event`, `task`, `action_item_list` types — these remain valid TOC leaf types; `topic_discussion` etc. are additions, not replacements

---

## Key implementation notes

**`TemplateRegistry::load` call sites:** The signature change from `load(dir)` to `load(dir, Option<&str>)` will break every existing call. There are call sites in at minimum `src/pipeline.rs` (test helpers), `src/template.rs` (tests), `src/prompt.rs` (tests), `src/main.rs`, and `src/mcp.rs`. Use `cargo build` errors to find them all. For test code that doesn't care about source family, pass `None`.

**`template_class` default:** The `TemplateClass` enum derives `Default` as `Utility`. This means any template file that doesn't have `template_class` in its frontmatter will load as `Utility` rather than failing to parse. This provides a safe fallback during the migration but should not be relied on — all templates must have `template_class` after Step 2.

**`floor_prompt` in `TemplateFrontmatter`:** Declared as `Option<String>`. Templates without `floor_prompt` (all non-content_unit types) will parse with `None`. Only `content_unit` templates should have `floor_prompt`.

**Pass 1 prompt placeholder:** The `{FLOOR_RULES}` placeholder must exist in `prompts/pass-1-toc-extraction.md` before Step 5's code change, or `build_pass1` will inject into a string with no placeholder (the `inject` function does a simple string replace and will silently do nothing if the placeholder is absent). Add the placeholder to the prompt file first, then update `build_pass1`.

**Test for `loads_all_fifteen_templates`:** This test asserts `registry.templates.len() == 16`. After this build, loading all families produces 21 templates; loading identity+utility only produces 13. The test name and assertion must be updated. Do not delete the test — update it to reflect the new selective-load behavior. A good replacement: one test for identity+utility-only load (`None` family), one for a meeting family load.

---

## Testing Checklist

- [ ] `cargo build` passes after template renames (Step 1) with no other changes
- [ ] `cargo build` passes after adding `template_class` frontmatter (Step 2)
- [ ] `cargo build` passes after creating new template files (Step 3)
- [ ] `cargo build` passes after all Rust changes (Steps 4–6)
- [ ] `cargo test` passes — all existing tests updated for new `load` signature
- [ ] Template registry loaded with `source_family = None` contains exactly the identity (7) + utility (6) templates = 13 total
- [ ] Template registry loaded with `source_family = Some("meeting")` contains identity + utility + `meeting-summary` + `meeting-topic-discussion` = 15 total
- [ ] `identity-person.md` registers with `entity_type = "person"` and `template_class = Identity`
- [ ] `meeting-topic-discussion.md` registers with `entity_type = "topic_discussion"` and `template_class = ContentUnit`
- [ ] `floor_prompt` field on `meeting-topic-discussion` is non-empty and parseable
- [ ] Ingesting a `meeting_summary` source: `TemplateRegistry` contains `meeting-*` + `identity-*` templates; does NOT contain `youtube_chapter` or `article_section`
- [ ] Ingesting a `youtube_video` source: registry contains `youtube-*` + `identity-*` templates; does NOT contain `topic_discussion` or `article_section`
- [ ] `build_pass1` for a meeting summary ingest: returned prompt string contains the `floor_prompt` text from `meeting-topic-discussion.md` (i.e., contains "topic_discussion leaves")
- [ ] `build_pass1` for a container source: returned prompt string does NOT contain "## Decomposition Floor Rules"
- [ ] `validate_no_floor_children` with a TOC containing a child leaf under a `topic_discussion` leaf returns `Err` with a message containing "content_unit floor type" and both addresses
- [ ] `validate_no_floor_children` with a valid flat TOC of `topic_discussion` leaves (no children) returns `Ok`
- [ ] Full ingest of a `meeting_summary` source file produces `topic_discussion`-typed notes via Pass 3 with no errors
- [ ] Existing `person`, `organization`, `context`, `task` entity types continue to work end-to-end — no regressions in existing test fixtures
- [ ] `%Atomicity.md` contains "Rule 3" and "Minimum Viable Granularity"

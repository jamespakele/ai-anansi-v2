---
name: scaffold-template
description: >
  Generate a properly-shaped anansi entity template file from a description
  of what the entity-type represents. Knows the schema - YAML frontmatter +
  `%%` field blocks + body template - the four template classes (identity,
  content_unit, source, utility), the three merge strategies (pure_atomic,
  container, source_bound), and the conventional field shapes per class.
  Returns a ready-to-drop-in `entity-[type].md` (or family-prefixed file)
  with sensible defaults the user can edit. Use when user says "scaffold
  a new template", "create a template for X", "/scaffold-template", or
  wants to add a new entity type to anansi without re-deriving the schema.
  Companion to `resource-typer` (which consumes templates) and
  `skill-description-tuner` (the other meta-tool in skill-builder.plugin).
argument-hint: "[entity_type name + a one-line description of what it represents]"
---

# scaffold-template

A meta-tool. Generate an anansi entity template file from a small input — entity_type name plus a one-line description of what it represents — and return a complete, drop-in-ready `.md` file with frontmatter, field blocks, and body.

Knows anansi's template schema cold so the user doesn't re-derive it every time.

---

## When to invoke

Trigger this skill when the user:

- Says "scaffold a new template", "create a template for [X]", "/scaffold-template"
- Wants to add a new entity type to anansi (a new `entity-<type>.md` or family-prefixed template)
- Pastes a description of what they want to track and asks for the template
- Hits a gap in the existing template set during a `resource-typer` or atomization session

Do not invoke for: editing an existing template (use Edit/Write directly), generating a SKILL.md (that's a different scaffold), or reorganizing the templates folder.

---

## Anansi's template schema

Every template has three parts:

1. **YAML frontmatter** between leading `---` and trailing `---`
2. **`%%` field blocks** — one per field, each with `field:` and `description:`
3. **Body template** — Markdown with `{{field_name}}` placeholders

### Frontmatter — required fields

```yaml
entity_type: <kebab-case-id>          # canonical type label
template_class: identity | content_unit | source | utility
atomic: true | false                  # does this create a stored note?
merge_strategy: pure_atomic | container | source_bound
template_version: "3.0"               # current version
description: "<one-line description>"
```

### Frontmatter — recommended fields

```yaml
atomic_criteria: >
  Multi-line statement of what this represents and what stays out
  (e.g., what content belongs in downstream notes instead).

identity_fields:
  name:
    type: string
    required: true
  <other_field>:
    type: string
    format: email | phone | prose
    description: "..."

sources:
  meeting_summary:
    hint: "When extracting from a meeting, look for X..."
  email_thread:
    hint: "..."
  research_paper:
    hint: "..."
  container:
    hint: "..."
```

### Frontmatter — container-only

```yaml
roster_sections:
  <section_key>:
    source_field: <section_key>
    render_as: "## <Heading>"
    row_format: "- {field1} — {field2}"
    dedupe_by: [field1, field2]
    description: >
      What this section accumulates, formatting rules, examples.
```

### Frontmatter — content_unit only (synergy floor types)

```yaml
floor_prompt: >
  Multi-line guidance to Pass 1 about when to create one of these and
  what stays inside vs. what spawns child notes. (Content units don't
  have children.)
```

### `%%` field blocks

One per field declared in `identity_fields`:

```
%%
field: name
description: <what to fill in here, written for the LLM doing extraction>
%%
%%
field: contact_email
description: Email address if mentioned in source
%%
```

### Body template

Markdown. Uses `{{field_name}}` for substitution. Section headings (`##`) for layout. For container types, the roster sections render below the body — leave just the heading and the merger fills it.

```markdown
# {{name}}

## Identity
- Field: {{field_value}}

## Section
{{another_field}}

## Roster Section            <!-- container only; merger appends bullets -->
```

---

## The four template classes — when to use each

### `identity` (atomic identity types)

The user's atoms. Person, organization, area, project, note. These persist over time and survive across documents. Identity-only — no narrative summaries or descriptions in the body. Time-varying data lives in edges or roster sections, not in the body.

- **`atomic: true`**
- **`merge_strategy: pure_atomic`** for stable identity (person, note)
- **`merge_strategy: container`** for active synergies with rosters (project, area, organization-with-context)
- Filename: `entity-<type>.md`

### `content_unit` (floor synergies)

Source-bound floor units that don't decompose further. Discussion, article-section, youtube-chapter, email-exchange. Have a `## Content` section with the narrative; sub-addresses become headings inside `## Content`, not separate notes.

- **`atomic: true`**
- **`merge_strategy: source_bound`**
- **`floor_prompt`** field required (tells Pass 1 not to decompose below this)
- Filename: family-prefixed (e.g., `meeting-topic-discussion.md`, `email-exchange.md`)

### `source` (non-atomic source documents)

The whole document being ingested. Meeting summary, email thread, research paper, YouTube video. Decompose into content_unit synergies. Never written as standalone notes — they spawn outline + leaves.

- **`atomic: false`**
- **`merge_strategy: source_bound`**
- Decomposes via Pass 1 into content_units of the matching family
- Filename: family-bare (e.g., `meeting-summary.md`, `email-thread.md`)

### `utility`

Special-purpose templates that don't fit the above. Outline (the source's MOC), container (generic source wrapper), event, task, action_item_list, context.

- **`atomic: true`** (mostly)
- **`merge_strategy: source_bound`**
- Filename: bare (e.g., `outline.md`, `task.md`)

---

## The three merge strategies — when to use each

| Strategy | Use when | Example |
|---|---|---|
| **`pure_atomic`** | Identity is stable; data doesn't accumulate in the body. Edges carry relationships. | person, note |
| **`container`** | Identity is stable but body has additive roster sections (members, contributors, stakeholders, context bullets). | project, area, organization (with context) |
| **`source_bound`** | Frozen-per-source. Each source ingest creates a fresh note keyed by `(source_id, toc_address)`. | discussion, article-section, task, event, outline, all source types |

---

## Process

When invoked:

1. **Gather inputs.** From the user's request, identify:
   - **`entity_type`** — kebab-case id (e.g., `place`, `book`, `recipe`)
   - **One-line `description`** — what does this represent?
   - **Template class** — identity? content_unit? source? utility?
   - **Merge strategy** — implied by class but ask if ambiguous
   - **Identity fields** — what data does each instance carry?
   - **Container/roster needs** — does the body accumulate anything?
   - **Source hints** — for which source-types should there be extraction hints?

   If something's unclear, ask the user. Use `AskUserQuestion` for decision points (template_class, merge_strategy, container yes/no).

2. **Pick the filename:**
   - `identity` → `entity-<type>.md`
   - `content_unit` / `source` → family-prefixed (`<family>-<type>.md`)
   - `utility` → bare (`<type>.md`)

3. **Render the template** following the schema above. Use sensible defaults:
   - `template_version: "3.0"`
   - For identity templates, default to lean: `name` always required; add user-specified fields; no `summary` / `content` unless the user asks (we explicitly avoid narrative bodies on atoms).
   - For content_unit templates, include `topic_sentence` and `content` fields as canonical.
   - For source templates, include `name`, `date_or_range`, `participants`, `overview`.

4. **Add `%%` field blocks** for every field in `identity_fields`. The block's `description` is what the LLM extractor reads when filling the field — write it as instructions to the extractor, not as user-facing docs.

5. **Render the body template** with `## Identity` / `## Content` / etc. sections, `{{field_name}}` placeholders, and roster headings (no body content under them) for container types.

6. **Verify:**
   - Frontmatter parses (no missing colons, indentation correct)
   - Every field in `identity_fields` has a `%%` block
   - Body uses `{{}}` placeholders for declared fields only
   - For container types, every `roster_sections` key appears as a body heading

7. **Output** the complete file content in a single fenced code block, ready to copy or write directly.

---

## Output format

```
## Scaffolded template: <filename>

**Class:** <identity | content_unit | source | utility>
**Merge strategy:** <pure_atomic | container | source_bound>
**Save to:** `<plugin-path>/templates/<filename>` or `source/templates/<filename>`

```markdown
---
entity_type: <type>
template_class: <class>
atomic: <bool>
merge_strategy: <strategy>
template_version: "3.0"
description: "<...>"

atomic_criteria: >
  <...>

identity_fields:
  name:
    type: string
    required: true
  <...>

[roster_sections, sources, floor_prompt — if applicable]
---
%%
field: name
description: <...>
%%
[other %% blocks]
%%
# {{name}}

## <Section>
{{field}}

[other sections]
```

## Notes

- <any decisions you made and why>
- <suggestions for refinement the user might consider>
- <flag any gaps you noticed in the user's input>
```

---

## Worked examples

### Example 1 — scaffold a `place` identity template

**Input:** "I want to track named places — restaurants, hotels, parks. Atomic identity. Need name, address, type-of-place, met_via (when did I first encounter it)."

**Output:**

```
## Scaffolded template: entity-place.md

**Class:** identity
**Merge strategy:** pure_atomic
**Save to:** `source/templates/entity-place.md` or `<plugin>/skills/<skill>/references/templates/entity-place.md`

```markdown
---
entity_type: place
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "3.0"
description: "A named location — restaurant, hotel, park, venue, building"

atomic_criteria: >
  Represents one named real-world place with durable identity. Identity-only
  — name, address, type, and connection origin. What happened at this place
  lives downstream in event/context notes; relationships to other entities
  flow through edges.

identity_fields:
  name:
    type: string
    required: true
  address:
    type: string
    description: "Street address or general location"
  type:
    type: string
    description: "Restaurant, hotel, park, museum, office, etc."
  met_via:
    type: string
    description: "How the user came to know this place. One short sentence."

sources:
  meeting_summary:
    hint: "Check meeting venue / location field"
  email_thread:
    hint: "Check signature blocks for office addresses; check 'meet at' phrases"
  research_paper:
    hint: "Check author affiliations for institutional addresses"
  container:
    hint: "Check any named locations referenced"
---
%%
field: name
description: The place's primary name as commonly used
%%
%%
field: address
description: Street address or general location, if mentioned
%%
%%
field: type
description: Category — restaurant, hotel, park, museum, office, etc.
%%
%%
field: met_via
description: One short sentence on how the user came to know this place. Smart-brevity style — name an introducing person, an event, a project. ≤15 words.
%%
# {{name}}

## Identity
- Address: {{address}}
- Type: {{type}}

## Met via
{{met_via}}
```

## Notes

- Followed the lean-identity pattern: no `summary` or `content` field. Narrative content (what you ate, what happened, etc.) belongs in downstream context/event notes that link here.
- If you want a running facts list (like organization's `## Context`), I can add a `roster_sections.context` block — switches `merge_strategy` to `container`. Say the word.
- `entity-place.md` slots into `resource-typer`'s `references/templates/` and into anansi's `source/templates/` as a fourth Resource subtype alongside person/organization/note.
```

### Example 2 — scaffold a `newsletter` source template

**Input:** "I want a source template for newsletters. Smart Brevity newsletters specifically — they have ranked items with '1 big thing' as the lead. Source type, decomposes into newsletter-item synergies."

**Output:**

```
## Scaffolded template: newsletter.md

**Class:** source
**Merge strategy:** source_bound
**Save to:** `source/templates/newsletter.md`

```markdown
---
entity_type: newsletter
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "3.0"
description: "A newsletter — recurring publication with multiple ranked items"

identity_fields:
  name:
    type: string
    required: true
  publication:
    type: string
    description: "Name of the recurring newsletter (e.g., Axios AM, 5 Big Things)"
  date:
    type: string
    description: "Publication date"
  author:
    type: string
    description: "Newsletter author or editor"
  overview:
    type: string
    description: "One-paragraph overview of the issue's themes"

sources:
  container:
    hint: "Newsletters arrive as a single document; treat the whole as the source. Items decompose into newsletter-item synergies, with '1 big thing' as the lead."
---
%%
field: name
description: The issue's title or subject (e.g., "Axios AM 2026-04-29")
%%
%%
field: publication
description: Name of the recurring publication
%%
%%
field: date
description: Publication date in ISO format (YYYY-MM-DD)
%%
%%
field: author
description: Author or editor of the issue
%%
%%
field: overview
description: One-paragraph overview describing what the issue covered
%%
# {{name}}

## Identity
- Publication: {{publication}}
- Date: {{date}}
- Author: {{author}}

## Overview
{{overview}}
```

## Notes

- Decomposes into `newsletter-item` content_unit synergies (you'll need that template too — say the word and I'll scaffold it).
- "1 big thing" is a Smart Brevity convention; the daemon's Pass 1 prompt should be told to label the lead item that way (a future prompt-tuning concern).
- No roster_sections — the items are full synergies, not body bullets.
```

---

## Edge cases

### "User wants a content_unit but doesn't say which family it belongs to"

Ask. Family prefix matters for filename and for source-family loading in the registry. (`meeting-`, `email-`, `youtube-`, `research-`, `newsletter-`, `speech-`, `presentation-`, `social-`, `company-update-`, etc.)

### "User wants a container template but no clear roster"

Push back. Container merge_strategy makes sense only if there's a roster section. If they just want identity fields with no accumulation, use `pure_atomic` instead.

### "User wants a source template but the source is too short to decompose"

Probably not a source then — it's an identity or content_unit. A short workplace memo doesn't really decompose; it IS one synergy. Suggest `memo.md` as a content_unit (or atomic identity, depending on whether memos accumulate context).

### "User wants `summary` and `content` fields on an identity template"

Push back gently. Identity templates are now lean — no narrative summaries, no content blobs in the body. Narrative belongs in downstream synergy notes that link to the identity. If they really want it (legacy reasons, specific user need), set those fields but flag the deviation in Notes.

### "User wants to deprecate / replace an existing template"

This skill scaffolds new templates; it doesn't manage deprecation. Suggest moving the old template to `templates/deprecated/` (anansi's convention — files there don't load).

---

## Anti-patterns to flag

### 1. Re-creating an existing type

Before scaffolding, ask whether anansi already has a similar template. If `entity-person.md` exists, don't scaffold `entity-individual.md`. The taxonomy stays clean only with discipline.

### 2. Over-fielded identity templates

Identity is stable. If the user wants 12 fields on a person template, most of them are probably edge / context content, not identity. Push back: identity is name + 1–3 stable attributes + connection origin. Everything else is downstream.

### 3. Naming with spaces or capitals

Filenames are kebab-case. Entity_types are kebab-case. Always lowercase, hyphen-separated. `entity-Customer-Org` would silently fail Cowork's auto-discovery.

### 4. Picking `container` because the user said "container"

The word "container" is overloaded. Cowork uses `template_class: utility` for the generic `container.md`. The merge_strategy `container` means "additive roster." Different concepts. Listen for which the user actually means.

---

## What this skill does NOT do (out of scope)

- **Does not write the file to disk.** It outputs the file content; the user (or downstream tooling) places it.
- **Does not validate against an existing template registry.** Doesn't know which templates already exist.
- **Does not generate body content beyond placeholders.** Doesn't make up sample data.
- **Does not maintain templates over time.** Edits to existing templates are the user's job.
- **Does not handle non-anansi templates.** This skill knows anansi's specific schema; other systems (Obsidian Templater, etc.) are out of scope.

---

## Why this matters

Adding a new entity_type to anan
---
name: anansi-new-entity-type
description: >
  Interactive guided workflow for creating a new anansi entity type. Queries
  the registry for existing types, walks the user through template class,
  merge strategy, identity fields, and source hints, then generates and saves
  the template file to the canonical location and syncs vendored mirrors.
  Builds on top of the scaffold-template schema knowledge. Use when the user
  says "create a new entity type", "add a new type", or "/anansi-new-entity-type".
argument-hint: "[entity_type name + optional one-line description]"
---

# anansi-new-entity-type

An interactive skill that guides the user through creating a new entity type
for the anansi knowledge graph. Unlike `scaffold-template` (which generates
a template and returns it as output), this skill is **end-to-end**: it checks
for conflicts, asks guided questions, writes the file to disk, and syncs
vendored copies.

---

## When to invoke

Trigger this skill when the user:

- Says "create a new entity type", "add a new type for X", "I need a template for Y"
- Uses `/anansi-new-entity-type`
- Identifies a gap during atomization ("we don't have a type for recipes")
- Wants to extend the anansi taxonomy with a new concept

Do **not** invoke for:
- Editing an existing template (edit the file directly)
- Listing entity types without creating one (use `anansi_list_entity_types` tool)
- Creating a SKILL.md (different scaffold)

---

## Process

### Step 1 — Check existing types

Call `anansi_list_entity_types` (MCP tool) to retrieve the current registry.

Display a summary table of existing types grouped by template class:

```
Current entity types (N total):

Identity:       person, organization, area, project, note, book, ...
Content Unit:   topic_discussion, email_exchange, youtube_chapter, ...
Source:         meeting_summary, email_thread, research_paper, ...
Utility:        context, event, task, action_item_list, outline, ...
```

Check whether the requested type already exists or overlaps with an existing
type. If it does, warn the user and ask whether they want to proceed anyway
or modify the existing template instead.

### Step 2 — Gather inputs

Ask the user the following questions (skip any already answered in their
initial request). Use the `AskUserQuestion` tool for multi-choice decisions.

1. **Entity type name** — kebab-case slug (e.g., `place`, `recipe`, `vehicle`).
   Validate: lowercase, no spaces, underscores OK for multi-word types.

2. **One-line description** — what does this entity represent?

3. **Template class** — explain each and ask:
   - `identity` — A durable real-world thing (person, org, place). Persists across documents.
   - `content_unit` — A bounded unit of meaning within a source (chapter, discussion, exchange). Floor type — no further decomposition.
   - `source` — A whole document being atomized (newsletter, meeting transcript). Decomposes into content_units.
   - `utility` — Flexible glue that doesn't fit above (event, task, context).

4. **Merge strategy** — defaults by class but confirm:
   - `identity` → `pure_atomic` (no body accumulation) or `container` (has roster sections)
   - `content_unit` → `source_bound` (always)
   - `source` → `source_bound` (always)
   - `utility` → `source_bound` (usually)

5. **Identity fields** — what data does each instance carry?
   - `name` is always required — don't ask about it
   - For each additional field ask: field name, type (string/number), format (email/phone/prose/bullets), required?, description
   - For identity types, push back on more than ~5 fields. Lean is better.

6. **Roster sections** (container merge_strategy only):
   - What accumulates in the body over time?
   - Section key, heading, row format, dedupe strategy

7. **Floor prompt** (content_unit only):
   - What tells the LLM to stop decomposing? Write the floor_prompt.

8. **Source family** (content_unit or source only):
   - Which family does this belong to? (meeting, email, youtube, research, newsletter, book, etc.)
   - This determines the filename prefix.

9. **Source hints** — for which document types should there be extraction hints?
   - At minimum: `meeting_summary`, `email_thread`, `container`/`generic_prose`

### Step 3 — Generate the template

Using the gathered inputs, generate the complete template file following the
schema documented in `scaffold-template`. The file has three parts:

1. **YAML frontmatter** — all required and applicable fields
2. **`%%` field blocks** — one per identity field
3. **Body template** — Markdown with `{{field_name}}` placeholders

### Step 4 — Confirm with user

Show the generated template to the user in a fenced code block. Ask for
confirmation or edits before writing to disk.

### Step 5 — Write and sync

1. **Determine filename:**
   - `identity` → `entity-<type>.md`
   - `content_unit` / `source` → `<family>-<type>.md`
   - `utility` → `<type>.md`

2. **Write to canonical location:**
   ```
   llm/plugins/anansi-config.plugin/references/templates/<filename>
   ```

3. **No mirrors needed.** Templates are loaded from the database at runtime.
   The disk copy in `anansi-config.plugin/references/templates/` is the single canonical location.

### Step 6 — Post-creation guidance

Tell the user:

> ✅ Template `<filename>` created in `llm/plugins/anansi-config.plugin/references/templates/`.
> Rebuild and restart the Anansi server, or run `anansi_reload_templates` if you maintain a DB override.
>
> **To make this available to the MCP server:**
> - The LLM skills (sb-atomize, para-resource-entities) will pick up the
>   new template immediately on next invocation.
> - The Rust MCP server loads templates at startup from the vault's
>   `templates_dir` (configured in `anansi.toml`). Run `anansi2 init` to
>   sync templates to your vault, then restart the server.
> - If the Rust binary embeds templates via `include_str!`, you'll need to
>   add a new const in `src/main.rs` and rebuild. This is only needed if
>   the template should be available for `anansi2 init` vault seeding.

---

## Anti-patterns to flag

### 1. Duplicating an existing type
If `person` already exists, don't create `individual`. Push back.

### 2. Over-fielded identity templates
Identity types should be lean: name + 1–3 stable attributes + connection
origin. Narrative content belongs in downstream source-bound notes.

### 3. Container without roster sections
If there's nothing to accumulate additively, use `pure_atomic` instead.

### 4. Content unit without a source parent
Every content_unit family needs a corresponding source template that
decomposes into it. If the source doesn't exist, scaffold both.

### 5. Non-kebab-case naming
Entity types use `snake_case` (e.g., `topic_discussion`, not `topic-discussion`).
Filenames use kebab-case (e.g., `meeting-topic-discussion.md`).

---

## Worked example

**User says:** "I want to track physical places — restaurants, offices, parks"

**Step 1:** Call `anansi_list_entity_types` → no `place` type exists.

**Step 2:** Gather:
- entity_type: `place`
- description: "A named physical location — restaurant, office, park, venue"
- template_class: `identity`
- merge_strategy: `pure_atomic` (no roster needed)
- identity_fields: name (required), address (string), type (string), met_via (string)
- source hints: meeting_summary, email_thread, container

**Step 3–4:** Generate and confirm:

```markdown
---
entity_type: place
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "3.0"
description: "A named physical location — restaurant, office, park, venue"

atomic_criteria: >
  Represents one named real-world place with durable identity. Identity-only
  — name, address, type, and connection origin. What happened here lives in
  downstream event/context notes; relationships flow through edges.

identity_fields:
  name:
    type: string
    required: true
  address:
    type: string
    description: "Street address or general location"
  type:
    type: string
    description: "Restaurant, office, park, museum, venue, etc."
  met_via:
    type: string
    description: "How the user came to know this place — one sentence"

sources:
  meeting_summary:
    hint: "Check meeting venue / location field"
  email_thread:
    hint: "Check signature blocks for office addresses; 'meet at' phrases"
  container:
    hint: "Check any named locations referenced"
toc_structure: "none"
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
description: Category — restaurant, office, park, museum, venue, etc.
%%
%%
field: met_via
description: One short sentence on how the user came to know this place. Smart-brevity style — ≤15 words.
%%
# {{name}}

## Identity
- Address: {{address}}
- Type: {{type}}

## Met via
{{met_via}}
```

**Step 5:** Write to `entity-place.md` in canonical + 2 mirrors.

**Step 6:** Display post-creation guidance.

---

## Relationship to other skills

- **scaffold-template** — this skill's output format matches scaffold-template's
  schema. anansi-new-entity-type is the guided end-to-end version.
- **para-resource-entities** — consumes identity templates at extraction time.
  New identity types appear in its type oracle automatically after vendored sync.
- **sb-atomize** — consumes all templates during atomization. New types are
  available to Pass 1 TOC generation and Pass 3 extraction after sync.
- **anansi_list_entity_types** — the MCP tool this skill calls in Step 1 to
  check for existing types.

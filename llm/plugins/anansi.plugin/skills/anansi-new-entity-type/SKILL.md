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

### Step 5 — Store in database and activate

The database is the shared communication channel between Claude Cowork and the
Anansi server. Templates are stored as `anansi_config` notes.

1. **Store the template via MCP** — call `anansi_capture` with these arguments:
   - `entity_type: "anansi_config"`
   - `name: "Template: <new_entity_type>"`
   - `match_key: "anansi_config:template:<new_entity_type>"`
   - `content: <full template markdown>` (the entire template with YAML frontmatter,
     `%%` field blocks, and body)
   - `lede: "Template definition for entity type '<new_entity_type>'"`

   > **Why `match_key` is required:** The name normalizer collapses colons to
   > hyphens, so without the override it would produce
   > `anansi_config:template-<type>` instead of the `anansi_config:template:<type>`
   > prefix the template registry expects. The `match_key` parameter is only
   > honored when `entity_type` is `anansi_config`.

2. **Hot-reload the server registry** — call `anansi_reload_templates` to make the
   new entity type immediately available. The server reads the template from the
   DB and parses it into its live registry — no restart needed.

3. **Verify** — call `anansi_list_entity_types` and confirm the new type appears.

4. **Optionally write locally** — if the user is working in the anansi source
   codebase and wants the template available for `anansi2 init` seeding, also
   write the file to:
   ```
   llm/plugins/anansi.plugin/references/templates/<filename>
   ```
   And sync to vendored mirrors:
   ```
   llm/plugins/anansi.plugin/skills/para-resource-entities/references/templates/<filename>
   llm/plugins/anansi.plugin/skills/sb-atomize/references/templates/<filename>
   ```

### Step 6 — Post-creation guidance

Tell the user:

> ✅ Template `<entity_type>` created and activated.
>
> **What happened:**
> - Stored as `anansi_config:template:<entity_type>` in the database
> - Hot-reloaded into the server's live template registry
> - The new type is immediately available for MCP tools and pipeline ingestion
>
> **LLM skill availability:**
> - The LLM skills (sb-atomize, para-resource-entities) will pick up the
>   new template from the database on next invocation.
> - If you also want this template in the source codebase for `anansi2 init`
>   seeding, let me know and I'll write it to the local templates directory.

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

**Step 3–4:** Generate and confirm (same as before).

**Step 5:** Store via MCP:
1. Call `anansi_capture` with:
   - `entity_type: "anansi_config"`
   - `name: "Template: place"`
   - `match_key: "anansi_config:template:place"`
   - `lede: "Template definition for entity type 'place'"`
   - `content: <full template markdown>`
2. Call `anansi_reload_templates` → server confirms it loaded the new template
3. Call `anansi_list_entity_types` → verify `place` appears in the identity types

**Step 6:** Display post-creation guidance.

---

## Relationship to other skills

- **scaffold-template** — this skill's output format matches scaffold-template's
  schema. anansi-new-entity-type is the guided end-to-end version.
- **para-resource-entities** — consumes identity templates at extraction time.
  New identity types appear in its type oracle automatically after DB reload.
- **sb-atomize** — consumes all templates during atomization. New types are
  available to Pass 1 TOC generation and Pass 3 extraction after reload.
- **anansi_list_entity_types** — the MCP tool this skill calls in Step 1 to
  check for existing types.
- **anansi_reload_templates** — the MCP tool this skill calls in Step 5 to
  hot-reload the template registry after creating a new type.


---
name: anansi-new-entity-type
description: >
  Interactive guided workflow for creating a new anansi entity type. First
  determines the entity's group (core/bootstrap or app-specific namespace),
  then walks through template class, merge strategy, identity fields, and
  source hints, then generates and saves the template file. The group prefix
  is applied automatically — the user never types it. Use when the user says
  "create a new entity type", "add a new type", or "/anansi-new-entity-type".
argument-hint: "[entity_type name + optional one-line description]"
---

# anansi-new-entity-type

An interactive skill that guides the user through creating a new entity type
for the anansi knowledge graph. Unlike `scaffold-template` (which generates
a template and returns it as output), this skill is **end-to-end**: it checks
for conflicts, asks guided questions, writes the file to disk, and syncs
vendored copies.

**Key design:** Every entity type belongs to a **group**. The group determines
the filename prefix and the `entity_type` namespace. Core/bootstrap types
(person, organization, project, area, note) use bare names. App-specific
systems (Coruscant, R2) use a `{group}_` prefix. The skill discovers existing
groups from disk and applies the prefix automatically.

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

### Step 0 — Determine the group

This step runs first, before any type checking. The group determines the
filename prefix and the `entity_type` namespace.

#### 0a — Scan for existing groups

Read the template directory at:
```
llm/plugins/anansi-config.plugin/references/templates/
```

Scan all `*.md` files. Discover groups by filename prefix patterns:

| Filename pattern | Group | Convention |
|---|---|---|
| `entity-*.md` | `core` | Bootstrap identity types — bare `entity_type` |
| `coruscant-*.md` | `coruscant` | App-specific — `coruscant_` prefix |
| `r2-*.md` | `r2` | App-specific — `r2_` prefix |
| `meeting-*.md`, `email-*.md`, `book-*.md`, etc. | *(pipeline families)* | Source/content_unit families — not user-selectable |
| `*.md` (bare) | `core` | Utility types — bare `entity_type` |

Present the discovered groups to the user:

```
Existing application groups:
  core       — Bootstrap types (person, org, project, area, note, event, ...)
  coruscant  — Coruscant flight system
  r2         — R2 drone system

Enter a group name, or type "new" to create a new group:
```

#### 0b — User picks a group

- **`core`** — The type gets a bare `entity_type` (e.g., `place`, `recipe`).
  Filename: `entity-{type}.md` for identity, `{type}.md` for utility.
- **App-specific group** (coruscant, r2, or a new one) — The type gets a
  `{group}_` prefix (e.g., `coruscant_flight`, `r2_drone`).
  Filename: `{group}-{type}.md`.

#### 0c — Create a new group (if needed)

If the user types a new group name that doesn't exist yet:

1. Ask for a one-line description of what the group represents
   (e.g., "Coruscant flight system — drone missions, operations, and flight tracking")
2. The group is born. No separate registry file needed — the group exists
   as soon as the first template with that prefix is created.
3. Record the group name and description for the post-creation guidance.

#### 0d — Derive the prefix

| Group | `entity_type` prefix | Filename prefix |
|---|---|---|
| `core` | *(none)* | `entity-` (identity) or bare (utility) |
| `{app}` | `{app}_` | `{app}-` |

The prefix is applied automatically in Step 5. The user never types it.

---

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
type. **Check within the group namespace** — `coruscant_flight` does not
conflict with a hypothetical `flight` core type, but `coruscant_flight` and
`coruscant_flight_output` are distinct.

If it does conflict, warn the user and ask whether they want to proceed
anyway or modify the existing template instead.

---

### Step 2 — Gather inputs

Ask the user the following questions (skip any already answered in their
initial request). Use the `AskUserQuestion` tool for multi-choice decisions.

1. **Entity type name** — the type within the group (e.g., `flight`, `recipe`,
   `drone`). Do NOT include the group prefix — it's applied automatically.
   Validate: lowercase snake_case, no spaces, no hyphens.

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
   - This determines the filename prefix (overrides the group prefix for pipeline families).

9. **Source hints** — for which document types should there be extraction hints?
   - At minimum: `meeting_summary`, `email_thread`, `container`/`generic_prose`

---

### Step 3 — Generate the template

Using the gathered inputs, generate the complete template file following the
schema documented in `scaffold-template`. The file has three parts:

1. **YAML frontmatter** — all required and applicable fields
2. **`%%` field blocks** — one per identity field
3. **Body template** — Markdown with `{{field_name}}` placeholders

The `entity_type` field in the frontmatter gets the full prefixed name:
- Group `core` + type `place` → `entity_type: place`
- Group `coruscant` + type `flight` → `entity_type: coruscant_flight`
- Group `r2` + type `drone` → `entity_type: r2_drone`

---

### Step 4 — Confirm with user

Show the generated template to the user in a fenced code block. Ask for
confirmation or edits before writing to disk.

---

### Step 5 — Write and sync

1. **Determine filename:**
   - `core` + `identity` → `entity-{type}.md`
   - `core` + `utility` → `{type}.md`
   - `{group}` (app-specific) → `{group}-{type}.md`
   - `content_unit` / `source` (pipeline family) → `<family>-{type}.md`

2. **Write to canonical location:**
   ```
   llm/plugins/anansi-config.plugin/references/templates/<filename>
   ```

3. **No mirrors needed.** Templates are loaded from the database at runtime.
   The disk copy in `anansi-config.plugin/references/templates/` is the single canonical location.

---

### Step 6 — Post-creation guidance

Tell the user:

> ✅ Template `<filename>` created in `llm/plugins/anansi-config.plugin/references/templates/`.
> Group: `{group}` — `{group_description}`
> entity_type: `{prefixed_entity_type}`
>
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

If a new group was created, add:

> **New group created:** `{group}` — `{description}`.
> Future types for this system will automatically use the `{group}_` prefix.
> To add more types to this group, run `/anansi-new-entity-type` again and
> select `{group}` from the existing groups list.

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

### 6. Missing app prefix for application-specific types
If a type belongs to a specific system (Coruscant, R2, etc.), it must carry
the app prefix to avoid namespace collisions with core types and other
systems. The group-based flow in Step 0 handles this automatically — never
let a user type `coruscant_flight` manually; they pick the `coruscant` group
and type `flight`, and the prefix is applied.

### 7. Core type in an app group
If a user tries to create a type like `person` or `organization` inside an
app group (e.g., `coruscant_person`), push back. Core identity types belong
in the `core` group. App-specific types should be things the app itself
introduces (flights, operations, drones, manifests), not duplicates of
bootstrap concepts.

---

## Worked example

**User says:** "I want to track physical places — restaurants, offices, parks"

**Step 0:** Scan templates → existing groups: core, coruscant. User picks `core`.

**Step 1:** Call `anansi_list_entity_types` → no `place` type exists.

**Step 2:** Gather:
- entity_type (within group): `place`
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

**Step 5:** Write to `entity-place.md` in canonical location.

**Step 6:** Display post-creation guidance.

---

**User says:** "I need a type for tracking Coruscant drone flights"

**Step 0:** Scan templates → existing groups: core, coruscant. User picks `coruscant`.

**Step 1:** Call `anansi_list_entity_types` → no `coruscant_flight` type exists.

**Step 2:** Gather:
- entity_type (within group): `flight`
- description: "A Coruscant flight — one atomic unit of drone work within an operation"
- template_class: `identity`
- merge_strategy: `pure_atomic`
- identity_fields: name (required), address, drone, objective, landing_zone, status, summary, content

**Step 3–4:** Generate and confirm — `entity_type: coruscant_flight` is applied automatically.

**Step 5:** Write to `coruscant-flight.md`.

**Step 6:** Display post-creation guidance with group: `coruscant`.

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
- **wiki-recall** — the local wiki recall skill that documents custom types
  by group. After creating a new group, consider updating wiki-recall's
  custom types table.

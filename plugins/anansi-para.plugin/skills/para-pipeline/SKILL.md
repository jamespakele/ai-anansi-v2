---
name: para-pipeline
description: >
  Orchestrator for the full PARA atomization pipeline and template management.
  Pipeline mode: accepts text or a file, runs para-extract → resource-typer →
  para-toc and returns a typed TOC note ready for anansi ingestion. Template
  management mode: list, add, or remove templates from references/templates/
  which drive entity typing and floor-type classification across the pipeline.
  Triggers: "run the para pipeline", "atomize this", "/para-pipeline",
  "process this document", "full para run", "extract and toc this",
  "list templates", "add template", "remove template", "what templates exist".
argument-hint: "[text content or file path to process] | [list|add|remove template]"
---

# para-pipeline

The PARA pipeline orchestrator. Two modes of operation:

1. **Pipeline mode** — orchestrates `para-extract` → `resource-typer` → `para-toc` in sequence, handling data handoff between steps and returning a formatted typed TOC note.
2. **Template management mode** — lists, adds, or removes templates from `references/templates/`, which is the type system driving resource-typer and para-toc across the entire pipeline.

---

## When to invoke

Trigger this skill when the user:

- Says "run the para pipeline", "/para-pipeline", "atomize this", "full para run"
- Says "extract and toc this", "process this document for anansi"
- Pastes a document and wants a typed TOC in one shot
- Has already run para-extract and resource-typer and just needs the toc step — skip to Step 3
- Says "list templates", "what templates are available", "show me the templates"
- Says "add template", "add a new template", "create a template"
- Says "remove template", "delete template", "drop template [name]"

Do not invoke for individual pipeline steps. Use `para-extract`, `resource-typer`, or `para-toc` standalone if the user wants to inspect or modify between steps.

---

## Inputs

- **Text or file** — raw document content (pasted inline or via file path). Required.
- **known_projects / known_areas** — optional vault context for matching existing entries. If provided, prepend to the para-extract input as a `---`-separated block.

---

## Pipeline

This skill is a pure orchestrator. Do **not** inline, summarize, or re-implement any
sub-skill logic. Read each sub-skill's SKILL.md in full and execute it exactly.
The sub-skill is always the source of truth — this file only wires them together.

**Finding sub-skill paths:** The base directory for this skill is provided in the invocation
context (the line beginning "Base directory for this skill:"). Resolve sibling paths by
replacing `para-pipeline` with the sub-skill name. Example: if the base directory ends in
`.../skills/para-pipeline`, then para-extract lives at `.../skills/para-extract/SKILL.md`.

---

### Step 1 — para-extract

1. Use the Read tool to load `../para-extract/SKILL.md`.
2. Execute para-extract **exactly** as its SKILL.md specifies, using the input document as the source.
   Pass `known_projects` / `known_areas` if provided.
3. Hold the full structured output (Projects, Areas, Resources, Concepts).
   Do not show it to the user unless they ask to see intermediate results.

---

### Step 2 — resource-typer

1. Use the Read tool to load `../resource-typer/SKILL.md`.
2. Execute resource-typer **exactly** as its SKILL.md specifies, using the Resources list
   from Step 1 as input.
3. Hold the full typed output.
   Do not show it unless asked.

---

### Step 3 — para-toc

1. Use the Read tool to load `../para-toc/SKILL.md`.
2. Execute para-toc **exactly** as its SKILL.md specifies, passing:
   - Projects, Areas, Concepts from Step 1
   - Typed Resources from Step 2
   - The original source document (for Discussion section structure)
3. Follow para-toc's output format, assembly, and reporting instructions completely.
   para-toc owns the final output — do not override or reformat it here.

---

## Intermediate results

By default, only the final TOC note is shown. If the user asks to see intermediate steps, show them after completing the full pipeline:

- **"Show me the para-extract output"** → display Step 1 output
- **"Show me the resource-typer output"** → display Step 2 output
- **"Show me just the TOC"** → display Step 3 output (default behavior)

---

## Report

After delivering the TOC note, print a one-line summary:

```
Pipeline complete — N projects · N areas · N discussion sections · N resources (Np persons, No orgs, Nn notes) · N concepts
```

Flag any unconfirmed resource types and any `existing` vault matches found.

---

## Template Management

Templates in `references/templates/` are the type system for the entire pipeline. `resource-typer` reads them to assign entity types; `para-toc` reads them to determine canonical alpha sections for floor types. Managing templates here propagates changes to all downstream skills automatically.

Templates come in two classes:
- **`identity`** — named entities that produce atomic notes (person, organization, area, project, note, etc.)
- **`content_unit`** — floor types that define how discussion sections are structured (meeting-topic-discussion, research-section, youtube-chapter, etc.)

---

### list-templates

**Triggers:** "list templates", "what templates are available", "show templates", "what types exist"

Read every `.md` file in `references/templates/`. For each, parse the YAML frontmatter and emit a summary table:

```
Templates in references/templates/ (N total)

Identity types (entity-*):
  entity_type          class      atomic  description
  ───────────────────────────────────────────────────────────────────
  person               identity   true    A unique individual — identity, contact, and connection origin
  organization         identity   true    A named group, company, NGO, government body, or institution
  note                 identity   true    Fallback for untyped content that doesn't fit a more specific type
  area                 identity   true    A domain of ongoing responsibility — PARA Area
  project              identity   true    A bounded effort with a clear outcome and deadline — PARA Project
  ...

Floor types (content_unit):
  entity_type              class          atomic  description
  ─────────────────────────────────────────────────────────────────────────
  topic_discussion         content_unit   true    A bounded discussion of a single topic within a meeting
  research-section         content_unit   true    A bounded section of a research paper or article
  youtube-chapter          content_unit   true    A chapter or segment of a YouTube video
  ...
```

End with: `Add a template with "add template" · Remove one with "remove template [entity_type]"`

---

### add-template

**Triggers:** "add template", "add a new template", "create a template", "new template"

1. Ask the user: what is the `entity_type` slug for the new template? (e.g., `decision`, `place`, `policy`)
2. Ask: is this an `identity` type or a `content_unit` (floor) type?
3. Ask: what does this type represent? (one sentence — becomes the `description` field)
4. For `content_unit` types, ask: what are the canonical alpha sections for this type? (e.g., "a. Key Points · b. Quotes · c. Takeaways")
5. Generate a valid template following the schema in `%Template-Schema.md`:
   - Required frontmatter: `entity_type`, `atomic`, `merge_strategy`, `template_version`, `description`
   - `template_class: identity` or `template_class: content_unit`
   - For `content_unit`: include `floor_prompt` and the canonical alpha sections
   - Populate `sources` hints for the four standard source types
   - Generate `%% field %%` blocks for the key identity fields
   - Generate a body template with `{{field_name}}` placeholders
6. Show the generated template to the user for review before writing
7. On confirmation, write to `references/templates/<entity_type>.md`
8. Report: "Template `<entity_type>` added. It is now available to resource-typer and para-toc."

---

### remove-template

**Triggers:** "remove template", "delete template", "drop template [name]"

1. If entity_type not specified, ask: which template to remove? (suggest running list-templates first)
2. Read the target template's frontmatter to confirm it exists and show its `description`
3. Warn if it is a core entity type (`person`, `organization`, `note`, `area`, `project`) — these are used as fallbacks across the pipeline. Require explicit confirmation ("yes, remove it") before proceeding.
4. On confirmation, delete `references/templates/<entity_type>.md`
5. Report: "Template `<entity_type>` removed. Existing TOC entries using this type are unaffected; future pipeline runs will no longer recognize it."

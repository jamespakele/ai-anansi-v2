# Anansi v2 — Full Specification

| Field | Value |
|---|---|
| Version | 1.0 |
| Date | 2026-04-23 |
| Status | Draft — awaiting implementation |
| Supersedes | `anansi-cowork-handoff.md` (v2.0), prior Anansi v1 codebase |
| Author | Pakele.ai / James Pakele |

---

## 0. Purpose of this Document

This is the canonical design specification for Anansi v2 — a clean-slate rewrite of the knowledge decomposition pipeline. It captures the complete data model, file formats, pipeline, merge semantics, database schema, and execution contracts needed to implement the Rust daemon, the Cowork plugin, and any future producer/consumer of the preprocessed-TOC format.

Everything Anansi v1 tried to do that isn't in this doc is intentionally out of scope for v2.

---

## 1. Purpose and Scope

### What Anansi v2 is

A knowledge decomposition pipeline that takes a source document (meeting notes, emails, research papers, transcripts) and produces a set of **typed atomic notes** in a flat Obsidian-compatible vault, linked by explicit edges stored in SQLite. The source becomes an outline (Map of Content); each leaf of the outline becomes a standalone atomic note reusable across sources.

### v1 scope

- Ingest one source at a time, produce outline + atomic leaf notes in a flat vault.
- Three LLM passes: Pass 1 (TOC extraction with enriched leaves), Pass 3 (node expansion), Pass 4 (relationship extraction). No Pass 1.5.
- Merge-aware from day one: three merge categories (pure atomic, container, source-bound) declared in templates.
- Preprocessed-TOC frontmatter schema as the contract for hybrid execution — Cowork produces TOCs for complex docs; Ollama/daemon handles the rest.
- Ollama as the default LLM backend on local GPU.
- Deterministic prompt assembly from templates + %Rules + source hints. No LLM calls for prompt generation.
- SQLite backing store colocated with the vault.
- Obsidian wikilinks throughout; graph-visualizable vault as the primary UX.

### Out of scope for v1

- Vector embeddings and hybrid semantic search. Simple SQL LIKE / FTS5 search only.
- Classical NLP preprocessing (spaCy-style NER). Deferred to v2+.
- File watcher / background daemon loop. Ingest is explicit (CLI + MCP tool).
- Recomposition, verification, audit layers.
- Cross-source entity disambiguation via LLM-assisted merge.
- Export to other formats.
- Multi-user or multi-vault operation.

---

## 2. Design Principles

Five commitments that govern every design choice:

**Atomicity through structure, not prompt discipline.** The vault's file layout, template merge rules, and entity-type separation enforce atomicity. No prompt says "don't contaminate the person note"; the person note literally has no place to store source-specific content. Purity is a data model property.

**Declarative over generative.** Templates declare what a note is; %Rules declare how the system behaves. Prompts are assembled deterministically from these. The pipeline never spends an LLM call to generate another prompt. (This replaces the Anansi v1 Pass 1.5 innovation, which was solving the right problem the expensive way.)

**One LLM call per meaningful unit of work.** Pass 1 = one call per document (judgment). Pass 3 = one call per leaf (extraction). Pass 4 = one call per document (relationships). Call count scales linearly with decomposition breadth, not with fanciness.

**The filesystem is the primary artifact.** Markdown files in an Obsidian-compatible vault are what the user owns and browses. SQLite (`anansi/web.db`) is the index; the web (`anansi/web/`) is the truth. Delete the DB, rebuild it from the web.

**Execution modes compose through files, not APIs.** Cowork produces a TOC by writing a file with specific frontmatter. The daemon consumes that file. Neither side calls the other. The disk is the message bus.

---

## 3. Architecture

### Data flow

```
Source document (.md)
    │
    ├── Optional: Cowork plugin produces enriched TOC
    │   (skill reads templates + %Rules from vault,
    │    authors TOC, splices into frontmatter,
    │    drops augmented file in ingest folder)
    │
    ▼
Daemon ingest
    │
    ├── [if anansi_toc in frontmatter]
    │      Validate TOC → skip Pass 1
    │
    └── [else] Pass 1 (Ollama) — generate enriched TOC
    │
    ▼
TOC spawn (no LLM) — parse leaves into instance nodes
    │
    ▼
Pass 3 (Ollama) — one call per leaf
    │   Deterministic prompt: template + %Rules +
    │   source_hint + leaf hint + source body
    │   Output: structured JSON (template fields + entities + roster)
    │
    ▼
Writer + Merger
    │   Atomic writes to vault; apply merge strategy per entity type
    │
    ▼
Pass 4 (Ollama) — one call per document
    │   Derive edges; write to DB
    │
    ▼
Complete — outline file written to outlines/, atomic notes flat in vault
```

### Execution modes

**Mode A — Daemon-only.** Drop source into ingest folder; daemon runs all passes against Ollama. Free, fast enough for small/simple docs.

**Mode B — Hybrid (Cowork TOC + daemon extraction).** For complex or important docs: Cowork skill runs Pass 1 using Claude Opus (via subscription) to produce the enriched TOC. Daemon picks up the augmented file, skips Pass 1, runs Pass 3 + Pass 4 on Ollama. Best quality where it matters, volume work on local GPU.

**Mode C — Manual.** User hand-authors a TOC in the source frontmatter, drops file into ingest folder. Daemon proceeds as in Mode B (preprocessed branch). Useful for curator work and re-ingestion after outline edits.

All three modes write identical output; they differ only in who authored the TOC.

---

## 4. Folder Layout

The metaphor: **anansi is the spider, the web is what it weaves.** The `anansi/` folder holds the whole system — config, rules, templates, sources, database. The `anansi/web/` folder holds everything anansi produces — outlines, atomic notes, context nodes. The user opens `anansi/web/` as their Obsidian vault and sees only the woven web, no system clutter.

```
anansi/
  anansi.toml                                  # config
  web.db                                       # SQLite — index + edges + contributions
  %Rules/
    %Atomicity.md                              # Rule 1 (Maximum Reusability)
    %Downstream-Flow.md                        # Rule 2 (Entity Purity)
    %Merge-Strategy.md                         # three-category merge rules
    %Template-Schema.md                        # template format reference
  templates/
    concept.md
    context.md
    event.md
    organization.md
    person.md
    project.md
    topic.md
    task.md
    area.md
    action_item_list.md
    note.md
    container.md                               # non-atomic source types
    email_thread.md
    meeting_summary.md
    research_paper.md
  {source-slug}.md                             # source files live at anansi root
  digital-futures-workshop-2026-01-30.md
  q3-planning-email-2026-04-22.md
  web/                                         # THE VAULT — user opens this in Obsidian
    {source-slug}.outline.md                   # outline MOC files (one per source)
    digital-futures-workshop-2026-01-30.outline.md
    -ian-kitajima.md                           # pure-atomic — flat at root of web/
    -burt-lum.md
    -pichtr.md                                 # container — flat at root of web/
    -sovereign-ai.md                           # concept
    -thriving.md
    -digital-futures.md                        # topic
    3-2-sovereign-ai-discussion-dfw-2026-01.md # source-bound — flat at root of web/
    6-1-digital-futures-workshop-2026-01-30.md
    7-1-draft-sovereign-ai-whitepaper.md
```

### Rationale

**Sources at `anansi/` root** — archival originals, outside the user's daily vault view. Preserved for re-ingest, audit, git history. Not wikilinked from the web (they're outside the vault), but their paths are recorded in `sources.source_path` and in each outline's frontmatter.

**Generated web at `anansi/web/`** — flat at this level. Outlines, atomic notes, container notes, source-bound notes all live side-by-side here. Wikilinks are globally addressed within this folder. Obsidian canon.

**Meta folders at `anansi/` root** (`%Rules/`, `templates/`) — system configuration, not user content. Daemon and Cowork skill read them from here. Never opened in Obsidian.

**Outline filename convention** — `{source-slug}.outline.md`. The `.outline.md` extension distinguishes MOC files from atomic notes when browsing `web/`. Obsidian treats `.md` as the file type; `.outline` is just part of the stem. Tools sort outlines together alphabetically and they're self-identifying.

### Entity-type prefixes in filenames (within `web/`)

| Prefix | Types | Example |
|---|---|---|
| `-` | person, organization, concept, topic, area | `-ian-kitajima.md`, `-pichtr.md`, `-sovereign-ai.md` |
| (none) | note | `some-note.md` |
| `{toc_addr}-` | context, event, task, action_item_list | `3-2-sovereign-ai-discussion-dfw-2026-01.md` |
| `.outline.md` suffix | outline | `digital-futures-workshop-2026-01-30.outline.md` |

### Source-bound naming

```
{toc_address}-{slugified-name}-{source-slug-short}.md
```

Example: `3-2-sovereign-ai-discussion-dfw-2026-01.md` — TOC address `3.2`, name "Sovereign AI discussion", source slug `dfw-2026-01`. Guarantees uniqueness across sources without a global namespace collision.

The `-` prefix on pure-atomic and container notes marks them as entities in the global namespace. Source-bound nodes carry their TOC address as a prefix for sortability and disambiguation. Outlines use the `.outline.md` suffix.

---

## 5. Entity Types and Merge Categories

### Three merge categories

Every entity type belongs to exactly one:

**Pure atomic.** Identity-only; never grows. First writer fills; later writers fill blanks only; conflicts logged to DB.
- `person`, `concept`, `topic`, `area`, `note`

**Container identity.** Identity zone is pure (treated as pure-atomic); plus one or more declared **roster sections** that accept additive set-union merges.
- `organization`, `project`

**Source-bound.** Write-once, keyed by `(source_id, toc_address)`. Never merged. Re-ingest of same source with changed content_hash regenerates the node in place.
- `context`, `event`, `task`, `action_item_list`

### Type inventory

Atomic output types (written to the vault and indexed as notes):

| Type | Category | Description | Parent |
|---|---|---|---|
| `concept` | pure-atomic | A single abstract idea or principle | — |
| `topic` | pure-atomic | A named subject area (no formal definition required) | — |
| `person` | pure-atomic | A unique individual | — |
| `area` | pure-atomic | A domain of ongoing responsibility (no end date) | — |
| `note` | pure-atomic | Fallback for untyped content | — |
| `organization` | container | Any named group — company, ngo, government, team | — |
| `project` | container | A named initiative with an end date | — |
| `context` | source-bound | A unit of interaction within an event | event |
| `event` | source-bound | A project with tight time bounds (hours–days) | project |
| `task` | source-bound | A single actionable item with one owner | — |
| `action_item_list` | source-bound | A cohesive set of tasks from one context | — |
| `outline` | source-bound | Map-of-Content for one source; renders its TOC with wikilinks to every leaf | — |

Note: `outline` is not "atomic" in the Zettelkasten sense (it's source-bound; you don't reuse it across sources), but it IS a first-class note — rows in the `notes` table, file on disk, participant in the edge graph, prime candidate for vector embeddings in v2. See §8.

Non-atomic source types (always decomposed; never written as atomic notes):

| Type | Description |
|---|---|
| `container` | Generic wrapper holding multiple sub-documents |
| `email_thread` | An email thread |
| `meeting_summary` | A meeting record |
| `research_paper` | An academic/research document |

### Temporal hierarchy

```
area          →  no end date            (ongoing)
project       →  has an end date        (weeks to years)
  └── event   →  tight time bounds      (hours to days)
        └── meeting (context)  →  tightest  (minutes to hours)
```

---

## 6. Templates

### Template file format

Each template is `templates/{entity_type}.md` — YAML frontmatter + `%% field %%` metadata blocks + body.

**Full example — `templates/organization.md` (container type):**

```yaml
---
entity_type: organization
atomic: true
merge_strategy: container
template_version: "2.0"
description: "A named group — company, ngo, government body, team"

atomic_criteria: >
  Represents one real-world organization with durable identity. Cannot be
  split without losing coherence. Personal roles and event-specific
  activities belong downstream.

identity_fields:
  name:
    type: string
    required: true
    description: "Primary org name as commonly used"
  full_name:
    type: string
    description: "Expanded or legal name if different"
  type:
    type: string
    description: "Company, NGO, government agency, research institution, etc."
  domain:
    type: string
    description: "Field of operation"
  summary:
    type: string
    description: "One-paragraph source-agnostic description"

roster_sections:
  people:
    source_field: people
    render_as: "## People"
    row_format: "- [[-{slug}|{name}]] — {role}"
    dedupe_by: [slug, role]

sources:
  meeting_summary:
    hint: "Check org name in intros, attendee list, 'from {org}' attributions"
  email_thread:
    hint: "Check From domain, signature blocks, company mentions"
  research_paper:
    hint: "Check author affiliations, funder acknowledgments, cited institutions"
---
%%
field: name
description: The primary organization name
%%
%%
field: full_name
description: Expanded or legal name if different from primary
%%
%%
field: type
description: Category of organization (company, NGO, government, etc.)
%%
%%
field: domain
description: Primary field of operation or focus area
%%
%%
field: summary
description: One-paragraph source-agnostic description
%%
# {{name}}

## Identity
- Full name: {{full_name}}
- Type: {{type}}
- Domain: {{domain}}

## Summary
{{summary}}
```

**Full example — `templates/person.md` (pure-atomic type):**

```yaml
---
entity_type: person
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "A unique individual"

identity_fields:
  name:
    type: string
    required: true
  contact_email:
    type: string
    format: email
  contact_phone:
    type: string
    format: phone
  summary:
    type: string
    description: "One-sentence source-agnostic description of who this person is"

sources:
  meeting_summary:
    hint: "Check attendee list, speaker attributions, intros — but do NOT capture what they said"
  email_thread:
    hint: "Check From headers, signature blocks. Do NOT capture email body content in the person note."
---
%%
field: name
description: Full name of the person
%%
%%
field: contact_email
description: Email address if mentioned in source
%%
%%
field: contact_phone
description: Phone number if mentioned in source
%%
%%
field: summary
description: One-sentence description — who is this person, independent of this source
%%
# {{name}}

## Contact
- Email: {{contact_email}}
- Phone: {{contact_phone}}

## Summary
{{summary}}
```

### Template frontmatter fields

**Required:**

| Field | Type | Description |
|---|---|---|
| `entity_type` | string | Matches filename stem. Used as lookup key. |
| `atomic` | bool | Whether this type produces atomic notes. `false` for source types. |
| `merge_strategy` | enum | `pure_atomic` \| `container` \| `source_bound` |
| `template_version` | string | Schema version for this template. Currently `"2.0"`. |
| `description` | string | Short human description. |

**Recommended:**

| Field | Type | Description |
|---|---|---|
| `atomic_criteria` | string | What makes this type atomic; when to split |
| `identity_fields` | map | Field definitions for the identity zone |
| `sources` | map | Per-source-type extraction hints |

**Container-only:**

| Field | Type | Description |
|---|---|---|
| `roster_sections` | map | Declared mergeable list sections. See below. |

**Roster section schema:**

```yaml
roster_sections:
  <section_key>:
    source_field: <pass3_output_key>       # where Pass 3 emits the list
    render_as: "## <markdown heading>"
    row_format: "- <template with {placeholders}>"
    dedupe_by: [<field1>, <field2>, ...]   # tuple uniqueness for set-union merge
```

### %% field blocks

Each `%% ... %%` block after frontmatter defines one field's extraction guidance:

```
%%
field: <field_name>
description: <what to extract>
format: <optional: bullets | prose | numbered | table>
constraints: <optional: rules the LLM should follow>
%%
```

These blocks are parsed into the `{TEMPLATE_FIELDS}` injection point in the Pass 3 prompt. They replace the v1 `field_blocks` structure with cleaner separation between metadata and body template.

### Body template

Everything after the last `%%` block is the body template. `{{field_name}}` placeholders are substituted at write time with the populated field values.

---

## 7. %Rules System

`%Rules/` is a folder of markdown files that encode system-wide behavior. They are loaded by the daemon at startup (and by the Cowork skill when it runs) and injected into prompts as ambient context.

### File format

Each rule file has simple YAML frontmatter plus markdown body:

```markdown
---
id: atomicity-rules
type: rule
status: active
version: "1.0"
last_updated: 2026-04-23
---

# Atomicity Rules

## Rule 1 — Maximum Reusability

Every atomic note must make full sense WITHOUT the source document in hand.

**The merge test:** If a second document mentions the same entity, can you
enrich the existing node rather than creating a new one? If yes, the note
passes. If it's so source-specific it would need rewriting, it failed.

## Rule 2 — Entity Purity and Downstream Flow

Each note contains ONLY information intrinsic to its entity type. Contextual
information flows downstream to context, event, project, or task nodes.

... (body continues)
```

### Loading

At startup, the daemon reads every `%*.md` file in `anansi/%Rules/`. Their contents are held in memory, keyed by filename stem (minus the `%` prefix). The Pass 1 and Pass 3 prompt builders reference these by name:

```rust
let atomicity = rules.get("Atomicity");
let merge = rules.get("Merge-Strategy");
```

### Prompt injection

Rules are not injected verbatim into every prompt — each prompt template declares which rules it wants via placeholders:

```
{RULES:Atomicity}
{RULES:Downstream-Flow}
```

The prompt builder substitutes these with the corresponding rule file's body. Rule files are versioned; the prompt template's expected rule version can be checked at load time.

### Canonical rule files for v1

- `%Atomicity.md` — Rule 1 and Rule 2 from the v1 handoff, refined.
- `%Downstream-Flow.md` — How context and source-specific content routes to context nodes.
- `%Merge-Strategy.md` — The three merge categories and how they compose.
- `%Template-Schema.md` — Reference documentation for template authors.

---

## 8. Outline as Map of Content (First-Class Note)

The outline is a first-class note — it gets a row in the `notes` table like any atomic note, participates in the edge graph, and is embedding-eligible. It differs from atomic notes only in that it's source-bound (one outline per source), not reusable, and lives in the `outlines/` subfolder rather than at the vault root.

### Why first-class

- **Searchable.** An outline is a dense structured summary of what a source contains. FTS5 queries for "sovereign AI" should hit both the atomic `-sovereign-ai.md` concept AND any outline whose source discussed it.
- **Embedding-ready.** An outline is typically 50–500 lines of grouped wikilinks — exactly the shape that vectorizes well for semantic similarity. When v2 adds embeddings, outlines become the cheapest, densest way to compute source-to-source thematic similarity. One small vector query answers "find docs similar to this workshop."
- **Graph-native.** Every leaf in an outline spawns a `contains` edge from the outline to the atomic note. Backlinks on `-ian-kitajima.md` show every outline whose source involved him — a natural "which docs is this person in" query without needing to scan source bodies.
- **Uniform schema.** Treating outlines as notes means one set of merge/write/query code paths, not two. `merge_category: source_bound` handles regeneration on re-ingest identically to context nodes.

### File format

```markdown
---
anansi_id: {uuid}
entity_type: outline
source_id: {source_id}
source: [[sources/{source-slug}]]
toc_author: {provenance-tag}
toc_generated_at: {iso-timestamp}
---

[[-Outline]]

# {Source Title} — Outline

## 1. Concepts
- 1.1 [[-thriving|Thriving]]
- 1.2 [[-aloha-os|Aloha as Operating System]]

## 3. Context
- 3.1 [[3-1-opening-welcome-dfw-2026-01|Opening welcome and thriving discussion]]
- 3.2 [[3-2-sovereign-ai-discussion-dfw-2026-01|Sovereign AI discussion — Session 3]]

## 4. People
- 4.1 [[-ian-kitajima|Ian Kitajima]]
- 4.2 [[-burt-lum|Burt Lum]]

... (all populated sections)
```

Sections are ordered 1–9 per the TOC schema. Each leaf is a bullet line with the TOC address and an Obsidian wikilink to the atomic note the leaf spawned.

### Match key and keying

Outlines are keyed one-to-one with their source: `match_key = "outline:{source-slug}"`. The `source_contributions` uniqueness constraint is satisfied by `toc_address = "0"` (sentinel) for the outline's own contribution row, leaving the decimal address space clean for leaf contributions.

### Edges spawned automatically

On Pass 1 completion, the daemon emits structural edges from the outline:

- `outline → contains → atomic_note` for every leaf the TOC spawned
- `outline → summarizes → source` (implicit via `source_id` FK; no edge row needed)

These are written during Pass 3 completion (once atomic notes exist to link to) as part of the same edge-insertion step Pass 4 uses, so the edge code path is unified.

In v2 when embeddings ship:

- `outline → similar_to → outline` via vector cosine above threshold — the cross-source thematic links that make the vault a real Zettelkasten

### Round-trip

The outline file IS a valid preprocessed-TOC source when combined with its source file. Edit an outline in Obsidian, correct a leaf's entity type or name, save, then re-ingest the source with updated outline leaves spliced into its `anansi_toc` frontmatter. The daemon processes the edited outline deterministically.

### Regeneration

Outlines are regenerated on every ingest (fresh Pass 1) or re-ingest (same source, content_hash changed). Re-ingestion of an unchanged source (same content_hash) is a no-op. The outline's `notes` row is updated in place — same `id`, updated `updated_at` — so edges pointing to it remain valid.

---

## 9. Preprocessed TOC Schema

The frontmatter contract by which any producer (Cowork skill, hand-authored, external tool) can submit a source with a pre-existing TOC, skipping Pass 1.

### Frontmatter fields

**Required:**

| Field | Type | Description |
|---|---|---|
| `anansi_toc_version` | integer | Schema version. Currently `1`. |
| `anansi_toc` | YAML literal block | The TOC text. One leaf per line. |

**Recommended:**

| Field | Description |
|---|---|
| `source_type` | Non-atomic source type (`meeting_summary`, `research_paper`, `email_thread`, `container`) |
| `title` | Human-readable source title |

**Optional:**

| Field | Description |
|---|---|
| `anansi_id` | Pre-allocated source UUID |
| `source_date` | ISO-8601 date for temporal anchoring |
| `toc_author` | Provenance tag: `ollama:<model>`, `claude-opus-4-7`, `codex:<model>`, `manual`, `daemon:pass_1` |
| `toc_generated_at` | ISO-8601 timestamp |
| `summary_1`, `summary_5` | Pre-populated source summaries |
| `tags` | List of freeform tags applied to source record |

### TOC leaf format

```
<address> <name> [<entity_type>] [| <key>: <value>]*
```

- `<address>` matches `\d+(?:\.\d+)+` — e.g. `1.1`, `4.2.3`
- `<name>` is free text; may contain em-dashes, colons, parens
- `<entity_type>` matches an entity type in the registry; case-sensitive
- Zero or more `| <key>: <value>` annotations, pipe-delimited

**Recognized annotations:**

| Key | Description |
|---|---|
| `hint` | 1–2 sentence summary of what the source says about this entity. Replaces Pass 1.5. |
| `context_at` | Comma-separated addresses where source-specific content about this entity routes to |
| `parent` | Explicit parent address if hierarchy can't be inferred |
| `flag` | `sparse`, `ambiguous`, or `duplicate` |

### Parser regex

```
^\s*
  (?P<address>\d+(?:\.\d+)+)
  \s+
  (?P<name>.+?)
  \s+
  \[(?P<entity_type>\w+|\?)\]
  (?P<annotations>(?:\s*\|\s*\w+:[^|]*)*)
\s*$
```

Lines not matching this regex are ignored (headers, blanks, comments).

### Validation

**Structural (failure → fall back to Pass 1):**
1. `anansi_toc_version` must equal a version the daemon understands
2. `anansi_toc` must parse to at least one valid leaf
3. Every address matches the decimal pattern
4. No duplicate addresses
5. Max address depth ≤ 6
6. Entity type exists in template registry OR equals `?`

**Semantic (warnings, not failures):**
7. `context_at` addresses should resolve to valid leaves in same TOC
8. Hint should be ≤ 300 characters
9. Source-type-matching template `sources.<source_type>.hint` should exist

### Daemon ingest behavior

1. Validate frontmatter per rules above
2. **If valid:** record `sources.preprocessed_toc = 1`, `sources.toc_author = <value>`, skip Pass 1, insert `toc_outline` node with provided TOC, proceed to TOC spawn
3. **If invalid:** log reason, fall back to Pass 1 (`toc_author = 'daemon:pass_1_fallback'`)
4. **Version mismatch:** attempt forward-compatible parse; fall back if fails

---

## 10. Database Schema

Four tables. SQLite with `PRAGMA foreign_keys = ON`.

```sql
CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    title TEXT,
    source_type TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    toc_hash TEXT,                          -- hash of anansi_toc if preprocessed
    preprocessed_toc INTEGER NOT NULL DEFAULT 0,
    toc_author TEXT,
    toc_generated_at TEXT,
    ingested_at TEXT NOT NULL,
    toc_text TEXT                           -- full TOC as ingested
);
CREATE INDEX idx_sources_hash ON sources(content_hash);

CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    name TEXT NOT NULL,
    match_key TEXT NOT NULL UNIQUE,         -- normalize(name, entity_type)
    file_path TEXT NOT NULL,                -- relative to anansi root (e.g. 'web/-ian-kitajima.md')
    summary_1 TEXT,
    summary_5 TEXT,
    merge_category TEXT NOT NULL,           -- 'pure_atomic' | 'container' | 'source_bound'
    created_from TEXT NOT NULL,             -- source_id that first created the note
    source_count INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_from) REFERENCES sources(id)
);
CREATE INDEX idx_notes_match ON notes(match_key);
CREATE INDEX idx_notes_type ON notes(entity_type);

CREATE TABLE source_contributions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    toc_address TEXT,
    hint TEXT,
    contribution_type TEXT NOT NULL,        -- 'created' | 'filled_fields' | 'added_roster' | 'regenerated' | 'conflict'
    payload TEXT,                           -- JSON: what was contributed (filled fields, added rows, conflict values)
    contributed_at TEXT NOT NULL,
    UNIQUE (source_id, note_id),
    FOREIGN KEY (source_id) REFERENCES sources(id),
    FOREIGN KEY (note_id) REFERENCES notes(id)
);
CREATE INDEX idx_contrib_source ON source_contributions(source_id);
CREATE INDEX idx_contrib_note ON source_contributions(note_id);

CREATE TABLE edges (
    id TEXT PRIMARY KEY,
    source_note_id TEXT NOT NULL,
    target_note_id TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    why TEXT,
    from_source TEXT NOT NULL,              -- source_id that revealed this edge
    weight REAL NOT NULL DEFAULT 1.0,
    metadata TEXT,                          -- JSON blob for extras (e.g. role for has_member)
    created_at TEXT NOT NULL,
    UNIQUE (source_note_id, target_note_id, edge_type, from_source),
    FOREIGN KEY (source_note_id) REFERENCES notes(id),
    FOREIGN KEY (target_note_id) REFERENCES notes(id),
    FOREIGN KEY (from_source) REFERENCES sources(id)
);
CREATE INDEX idx_edges_source ON edges(source_note_id);
CREATE INDEX idx_edges_target ON edges(target_note_id);
CREATE INDEX idx_edges_type ON edges(edge_type);
```

### Conflicts table (optional, v1.5)

If the `## Conflicts` UX proves insufficient, a dedicated conflicts table:

```sql
CREATE TABLE field_conflicts (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    field TEXT NOT NULL,
    value_existing TEXT,
    value_new TEXT,
    source_existing TEXT,
    source_new TEXT,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
```

v1 records conflicts in `source_contributions.contribution_type = 'conflict'` with payload JSON. v1.5 promotes them to their own table if the query pattern justifies it.

---

## 11. Pipeline

### Pass 1 — TOC Extraction

- **Input:** source body, entity types registry, %Rules, source_type
- **Output:** enriched TOC (leaves + hints + context_at); persisted as the source's `anansi_toc` text, the `outline` note row (in `notes` table), and the outline markdown file at `outlines/{source-slug}.md`
- **LLM calls:** 1 per source
- **Skipped when:** source frontmatter already has a valid `anansi_toc`

Prompt composition (deterministic):
```
[RULES:Atomicity]
[RULES:Downstream-Flow]

[ENTITY_TYPES]

[Leaf format specification]

[Source body]
```

### TOC Spawn (no LLM)

- Parse each leaf line with the documented regex
- Extract address, name, entity_type, hint, context_at
- Insert one `instance_outline` record per leaf with `pass_stage = 'pass_3_pending'`

### Pass 3 — Node Expansion

- **Input:** one leaf (address, name, entity_type, hint, context_at), template for that entity_type, source_hint from template, %Rules, source body
- **Output:** structured JSON with template fields, optional roster (for container types), entities list, summary_1, summary_5, tags
- **LLM calls:** 1 per leaf
- **Mode:** JSON output mode (Ollama `format: "json"`)

Prompt composition (deterministic):
```
[RULES:Atomicity]

You are extracting one {entity_type} note: {name}

[Source hint for {source_type}]

[Leaf hint from TOC]

[Template fields to extract]

[context_at routing: content about these topics belongs downstream at {addresses}]

[Source body]

[Output schema]
```

Expected output shape:

```json
{
  "fields": {
    "name": "...",
    "role": "...",
    ...
  },
  "roster": {
    "people": [
      {"name": "...", "slug": "...", "role": "..."},
      ...
    ]
  },
  "summary_1": "...",
  "summary_5": "...",
  "tags": ["...", "..."],
  "entities": [
    {"name": "...", "entity_type": "...", "slug": "..."}
  ]
}
```

For pure-atomic types, `roster` is absent. For source-bound types with an `## Entities` section (context, event), `entities` is rendered directly into the body.

### Pass 4 — Relationship Extraction

- **Input:** all Pass 3 outputs for a source, the TOC, the implicit edges derived from TOC structure
- **Output:** list of explicit edges not already captured structurally
- **LLM calls:** 1 per source
- **Mode:** JSON output mode

Edges derived without LLM (structural):
- Person under org section X.Y → `member_of` → parent
- Context under event X.Y → `belongs_to` → parent
- Task with `— {assignee}` suffix → `assigned_to` → named target
- TOC cross-reference `[→ X.Y]` → `related_to`

Edges requiring LLM (semantic):
- Concept discussed in context → `discusses`
- Project depends on project → `depends_on`
- Source contradicts source → `contradicts`
- Thematic connections the TOC doesn't reveal

### Canonical edge types

```
member_of       belongs_to      contains        related_to
discusses       involves        produced        assigned_to
part_of         depends_on      references      addresses
led_by          funded_by       supports        contradicts
extends         precedes        follows         has_member
```

The `has_member` edge carries `metadata: {"role": "President"}` for roster relationships.

---

## 12. Merge Semantics (Detailed)

### Pure-atomic merge

```
on ingest(new_note):
  existing = lookup_by_match_key(new_note.match_key)
  if not existing:
    create_file_and_db_row(new_note)
    contribution_type = 'created'
  else:
    for each field in new_note.identity_fields:
      if existing[field] is blank or [not mentioned]:
        existing[field] = new_note[field]
        mark_contribution('filled_fields', field)
      elif existing[field] != new_note[field]:
        record_conflict(field, existing[field], new_note[field])
        mark_contribution('conflict', field)
      else:
        # no-op; same value
    rewrite_file_with_merged_identity(existing)
    increment source_count
  insert source_contributions row
```

### Container merge

```
on ingest(new_note):
  existing = lookup_by_match_key(new_note.match_key)
  if not existing:
    create_file_and_db_row(new_note, including roster sections)
    contribution_type = 'created'
  else:
    # 1. Identity merge — same as pure-atomic
    merge_identity_fields(existing, new_note)

    # 2. Roster merge — additive set union
    for each roster_section in template.roster_sections:
      existing_rows = parse_section(existing.body, section.render_as)
      new_rows = new_note.roster[section.source_field]
      merged = set_union(existing_rows, new_rows, dedupe_by=section.dedupe_by)
      rewrite_section(existing.body, section.render_as, merged, section.row_format)

    # 3. Edges — write one edge per new row added
    for each row added to roster:
      write_edge(container → member, edge_type, metadata={role, ...})

    contribution_type = 'added_roster' or 'filled_fields'
    increment source_count
  insert source_contributions row
```

### Source-bound merge

```
on ingest(new_note):
  existing = lookup_by (source_id, toc_address)
  if not existing:
    create_file_and_db_row(new_note)
    contribution_type = 'created'
  else:
    if new_note.source.content_hash == existing.content_hash:
      # no-op — unchanged
      return
    else:
      overwrite_file_with_new_content(existing.file_path, new_note)
      contribution_type = 'regenerated'
  insert source_contributions row
```

### match_key normalization

```rust
fn normalize(name: &str, entity_type: &str) -> String {
    let normalized = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>();
    let collapsed = normalized.split_whitespace().collect::<Vec<_>>().join("-");
    format!("{}:{}", entity_type, collapsed)
}
```

Examples:
- `normalize("Ian Kitajima", "person")` → `"person:ian-kitajima"`
- `normalize("PICHTR", "organization")` → `"organization:pichtr"`
- `normalize("Sovereign AI", "concept")` → `"concept:sovereign-ai"`

Collisions (two different humans named "John Smith") use first-writer-wins; manual `/fork` command in v1.5 handles disambiguation when needed.

---

## 13. Slug Generation and Filenames

```rust
fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn file_path(entity_type: &str, name: &str, toc_address: Option<&str>, source_slug: Option<&str>) -> String {
    let base = slug(name);
    match merge_category_of(entity_type) {
        PureAtomic | Container => format!("-{}.md", base),
        SourceBound => {
            let addr = toc_address.unwrap().replace('.', "-");
            let src = source_slug.unwrap();
            format!("{}-{}-{}.md", addr, base, src)
        }
    }
}
```

The `note` type (plain, no `-` prefix) is an exception:

```rust
PureAtomic if entity_type == "note" => format!("{}.md", base),
```

---

## 14. LLM Backend

### Default: Ollama

```toml
[llm]
backend = "ollama"
url = "http://localhost:11434"
model = "qwen2.5:14b"
n_ctx = 16384
timeout_s = 600

[llm.decomposition]
temperature = 0.2
max_tokens = 4096
json_mode = false

[llm.extraction]
temperature = 0.2
max_tokens = 4096
json_mode = true      # Pass 3, Pass 4

[llm.synthesis]
temperature = 0.5
max_tokens = 8192
json_mode = false
```

### Per-pass routing

```toml
# Optional — route specific passes to alternate backends
# [llm.per_pass.pass_1]
# backend = "codex-cli"
# model = "gpt-5"

# [llm.per_pass.pass_4]
# backend = "claude-cli"
# model = "opus"
```

### Supported backends (v1)

| Backend | How it works |
|---|---|
| `ollama` | HTTP to local Ollama server; `format: "json"` for JSON mode |
| `codex-cli` | Shells out to `codex exec`, stdin piped |
| `claude-cli` | Shells out to `claude -p`, stdin piped |

---

## 15. Cowork Plugin Contract

The Cowork plugin is a producer of preprocessed-TOC-formatted source files. It does NOT call the daemon directly; it writes augmented files to a configured path.

### Plugin structure

```
anansi.plugin/
  .claude-plugin/plugin.json
  skills/
    anansi-toc/
      SKILL.md              # orchestration: read source, produce TOC, splice, save
  commands/
    toc.md                  # /toc <source-file>
```

Templates and %Rules are read from the user's vault at runtime (configured path). The plugin ships no templates — one canonical source of truth lives in the vault.

### Skill responsibilities

1. Read the source file from the path the user provides
2. Read templates from `<anansi-root>/templates/*.md`
3. Read `<anansi-root>/%Rules/%Atomicity.md` and `<anansi-root>/%Rules/%Downstream-Flow.md`
4. Assemble the Pass 1 prompt deterministically (same way the daemon would)
5. Claude reasons through the prompt, produces an enriched TOC conforming to the schema
6. Splice the TOC into the source's frontmatter as `anansi_toc`
7. Set `toc_author` to a truthful provenance tag (e.g., `claude-opus-4-7`)
8. Set `toc_generated_at` to the current ISO timestamp
9. Write the augmented file to the anansi root (the daemon picks it up from there)
10. Report the output path to the user; optionally offer a bash call to trigger daemon ingest

### Skill MUST NOT

- Modify the source body
- Pre-allocate `anansi_id` without coordinating with the daemon
- Emit a TOC that violates the schema (producing fallback = wasted work)
- Assume the daemon is running

---

## 16. Module Layout and LOC Targets

```
anansi2/
  Cargo.toml
  src/
    main.rs              # ~80 lines  — CLI: ingest, list-sources, show-note
    config.rs            # ~60  — TOML loader
    db.rs                # ~350 — four tables, CRUD, migrations embedded
    vault.rs             # ~140 — paths, slugify, wikilink rendering
    template.rs          # ~230 — YAML + %% block parser, merge_strategy parsing
    rules.rs             # ~70  — load %*.md files, named lookup
    llm.rs               # ~120 — Ollama HTTP, JSON mode, timeout
    prompt.rs            # ~180 — deterministic prompt assembly
    pipeline.rs          # ~260 — Pass 1 (skippable), Pass 3, Pass 4 orchestrator
    writer.rs            # ~200 — frontmatter, wikilinks, atomic writes, section rendering
    merger.rs            # ~180 — identity merge, roster union, source-bound regeneration
    mcp.rs               # ~250 — tools: ingest, search, get, edges
  migrations/
    0001_schema.sql
```

**Total: ~2,120 lines of Rust.**

### Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
axum = "0.7"
clap = { version = "4", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
regex = "1"
once_cell = "1"
slug = "0.1"
async-trait = "0.1"
```

Drops from Anansi v1: `llama-cpp-2`, `fastembed`, `notify`, `encoding_rs`, `sha2` (content-hash can use a small wrapper or skip v1), `unicode-normalization`.

---

## 17. What's Deferred

Explicitly out of v1 scope, deferred to v1.5 / v2:

- **Full-text search.** v1.5 adds FTS5 virtual table over notes. Basic SQL LIKE search in v1 is sufficient up to ~5K notes.
- **Vector embeddings and hybrid search.** v2. Requires picking an embedding model, storing vectors, implementing RRF. Outlines are the prime first targets to embed — they're already structured summaries, dense in semantics, and give cross-source thematic similarity with one vector per source. Atomic notes embed their `summary_5` as the second wave.
- **Classical NLP preprocessing.** v2+. Shell out to spaCy or use `rust-bert` for NER-first, LLM-only-for-enrichment pattern.
- **File watcher daemon loop.** v1.5 if needed. v1 is explicit ingest via CLI/MCP.
- **Recomposition (distilled synthesis from a note + its edges).** v2.
- **Verification pass.** v2.
- **LLM-assisted entity disambiguation on merge.** v2. v1 uses first-writer-wins + manual `/fork`.
- **Multi-user / shared vault.** Not planned.
- **Export to other formats.** v2 if users want.

---

## 18. Extension Strategy

### Adding a new entity type

1. Create `templates/<entity_type>.md` with appropriate `merge_strategy`
2. If `merge_category: container`, declare `roster_sections`
3. If it has a parent type hierarchy, set `parent_type`
4. Optionally add source_hints for each source_type
5. Restart daemon — picked up automatically

No Rust changes required for new types.

### Adding a new rule

1. Create `%Rules/%<RuleName>.md`
2. Reference it from a prompt template via `{RULES:<RuleName>}`
3. Restart daemon

### Adding a new producer

Any tool that can write a valid preprocessed-TOC frontmatter file plugs in at the boundary. Cowork plugin is one example; Codex CLI wrapper, hand-authoring, external ingestion tools all work the same way.

### Schema version bumps

**Template schema:** `template_version` in frontmatter. v2.0 current. Backward-compat rules: additive fields preserve compatibility; removing or renaming fields requires a major version bump and a daemon update.

**TOC frontmatter schema:** `anansi_toc_version`. v1 current. Same rules.

**Database schema:** `sqlx::migrate!` handles forward migration automatically on startup.

---

## 19. Open Questions

Questions from the initial draft — tracking their resolution:

1. ~~**Should `outlines/` files be treated as atomic notes?**~~ **Resolved:** outlines are first-class notes — rows in the `notes` table, `merge_category: source_bound`, embedding-eligible in v2, spawn `contains` edges to their leaves. See §8.

2. **Should source files be the original or the augmented (post-ingest) version?** **Decision: try augmented in v1, revisit.** The augmented file (source body + `anansi_toc` frontmatter from Pass 1 or from a producer) sits at the anansi root. Re-ingestion reads the augmented file and decides whether the TOC needs regeneration based on `content_hash` vs. `toc_hash`. The original can be preserved in git if desired. If this produces friction, v1.5 can introduce a `sources/originals/` convention.

3. ~~**Body template placeholder escaping.**~~ **Resolved (provisional):** escape `{` as `\{` in field values during Pass 3 output parsing; the template renderer unescapes `\{` to `{` on body write. v2 can adopt a stricter template engine (handlebars, tinytemplate) if this becomes a real problem in practice.

4. ~~**Obsidian graph view exclusions.**~~ **Resolved by folder layout:** the user opens `anansi/web/` as the Obsidian vault. Sources, `%Rules/`, `templates/`, `web.db`, and config all live at the anansi root — outside the vault entirely. Nothing to exclude because the vault only contains generated files. See §4.

---

## 20. Implementation Order

Suggested build order for minimum time to working pipeline:

1. **Cargo.toml + migrations + `db.rs`** — schema locked, can hand off rest to Claude Code
2. **`template.rs` + one template (`person.md`, `organization.md`, `concept.md`, `context.md`)** — parser working end-to-end on four types
3. **`rules.rs` + canonical %Rules files** — ambient context wired up
4. **`vault.rs` + `writer.rs`** — file writing, wikilink resolution, slug generation
5. **`llm.rs` (Ollama only)** — validate against `ollama pull qwen2.5:14b`
6. **`prompt.rs`** — deterministic assembly from templates + %Rules + source hints
7. **`pipeline.rs`** — wire Pass 1 → TOC spawn → Pass 3 → Pass 4
8. **`merger.rs`** — three-category merge logic
9. **`mcp.rs` + `main.rs`** — ingest tool + CLI entry
10. **Cowork plugin** — thin producer for the preprocessed-TOC format
11. **End-to-end test with real source** (a past meeting note from the v1 vault) — verify output matches the spec

Each step is testable in isolation; the pipeline comes together at step 7.

---

## 21. Deployment

### v1 — local Docker on the GPU machine

The initial deployment target. The container runs the anansi2 binary; Ollama runs directly on the host (not containerized); the anansi root is bind-mounted from the host so the vault and database persist across container restarts. User opens `./anansi/web/` in Obsidian on the host.

**`Dockerfile`** (~30 lines, two-stage build, debian-slim runtime):

```dockerfile
FROM rust:1.80-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/anansi2 /usr/local/bin/anansi2
EXPOSE 3738
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD curl -f http://localhost:3738/health || exit 1
VOLUME ["/anansi"]
ENTRYPOINT ["anansi2", "serve", "--root", "/anansi"]
```

**`docker-compose.yml`** (~20 lines):

```yaml
services:
  anansi:
    build: .
    container_name: anansi
    restart: unless-stopped
    ports:
      - "3738:3738"
    volumes:
      - ./anansi:/anansi
    environment:
      - ANANSI_OLLAMA_URL=http://host.docker.internal:11434
      - ANANSI_OLLAMA_MODEL=qwen2.5:14b
      - RUST_LOG=info
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

**Additions to the Rust for Docker support:**

- `GET /health` endpoint in `mcp.rs` (~10 lines) returning `{"status":"ok","ollama":"reachable","db":"ok"}` after pinging Ollama and the DB. Used by the Dockerfile HEALTHCHECK; also useful for monitoring.
- Env-var overrides in `config.rs` for `ANANSI_OLLAMA_URL` and `ANANSI_OLLAMA_MODEL` (~15 lines) — standard 12-factor pattern, lets docker-compose configure the container without rebuilding.

Total v1 Docker cost: **~25 lines of Rust + ~55 lines of config**. Fits inside the existing `mcp.rs` and `config.rs` module budgets.

### v2 — VPS migration (deferred, no GPU available)

When anansi moves to a GPU-less VPS, three things change:

**Image slims.** Switch to Alpine base (musl target) for a ~40MB image:
```dockerfile
FROM rust:1.80-alpine AS build
RUN apk add --no-cache musl-dev openssl-dev
...
FROM alpine:3.19
RUN apk add --no-cache ca-certificates curl
```

**Pass 1 moves to Cowork.** Without a GPU, running Pass 1 against Ollama on the VPS is impractical. The hybrid mode from §3 becomes the default path: Cowork skill preprocesses the TOC using the Claude Max subscription, splices it into the source's frontmatter, submits the augmented file. The daemon skips Pass 1 every time and only runs Pass 3 + Pass 4.

**Pass 3/4 routing decision** (open until the move). Options:
- Per-pass backend routing to a Codex or Claude CLI shim on the VPS (patterns already covered in §14).
- A cloud Ollama-compatible provider (Groq, Together.ai) as a new backend — small addition to `llm.rs`.
- Direct Anthropic API calls on a pay-per-token basis — accept the bill for the low-volume extraction work.

Decide at migration time based on cost/latency measured against your actual ingestion rate.

### Home → VPS migration recipe

1. Stop the home container: `docker compose down`.
2. `rsync -a ./anansi/ vps:~/anansi/` — the vault is portable (markdown files + SQLite).
3. On the VPS, pull the Alpine image and start: `docker compose up -d`.
4. Update `anansi/anansi.toml` on the VPS to reflect the new backend routing (per §14 per-pass overrides).
5. Configure the Cowork plugin to point at the VPS's MCP endpoint if you want ingest-trigger from Cowork; otherwise use `anansi2 ingest` over SSH.

Optional: run a **read-only VPS** where ingestion stays on the home machine and the VPS is query/search only. Add a `read_only = true` config flag that disables the ingest MCP tool — ~5 lines in `mcp.rs`. This lets the VPS serve search/graph queries to any client (Cowork on any device, Obsidian plugins, mobile) without the VPS needing to do any LLM inference.

### Portability guarantees

The anansi root is self-contained:
- `anansi.toml` — config
- `%Rules/` — behavior
- `templates/` — schemas
- `web/` — all generated notes
- `web.db` — the index
- `{source}.md` files — the originals

Move the folder, the whole system moves with it. No paths outside the anansi root, no hidden state, no registry entries. Migration between home, VPS, another machine, or a fresh Docker volume is `rsync` + `docker compose up`.

---

*Anansi v2 · Spec v1.0 · 2026-04-23 · Pakele.ai*

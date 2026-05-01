# Anansi v2 — Smart Brevity Integration Handoff

**Prepared for:** Development agents  
**Date:** 2026-04-30  
**Author:** Claude (Sonnet 4.6) via design session with James Pakele  
**Project root:** `ai-anansi-v2/`  
**Source code:** `ai-anansi-v2/source/`  
**Resources:** `ai-anansi-v2/resources/`

---

## 1. What This Document Is

This is a complete design handoff for integrating Smart Brevity into the Anansi v2 knowledge pipeline. It covers the reasoning behind every decision, the existing codebase architecture, the new workflow, schema migrations, and a prioritized implementation task list.

Do not implement anything without reading sections 2–6 first. The decisions in sections 7–12 depend on understanding the reasoning, not just the outputs.

---

## 2. Background: Anansi v2 Existing Architecture

### 2.1 What Anansi Does

Anansi is a personal knowledge vault system. It ingests source documents (emails, meeting notes, books, articles, transcripts) and atomizes them into a typed knowledge graph of individual notes connected by edges.

The design principle: every piece of information should be retrievable as a small, self-contained atomic note — not buried inside a large document.

### 2.2 Existing Codebase Stack

**Language:** Rust  
**Database:** SQLite via SQLx  
**LLM integration:** Ollama, Gemini, OpenRouter (switchable via config)  
**MCP server:** Axum HTTP server, JSON-RPC 2.0  
**Key dependencies:** `sqlx`, `axum`, `tokio`, `uuid`, `sha2`, `serde_json`

**Key source files:**

| File | Purpose |
|---|---|
| `src/main.rs` | CLI: `init`, `ingest`, `serve` subcommands |
| `src/db.rs` | SQLx structs, all DB queries |
| `src/pipeline.rs` | Full ingestion pipeline (1098 lines) |
| `src/merger.rs` | Merge/conflict logic for duplicate notes |
| `src/mcp.rs` | MCP server: `anansi_ingest`, `anansi_search`, `anansi_get`, `anansi_edges`, `anansi_relate` |
| `src/vault.rs` | Vault path conventions |
| `src/template.rs` | Template registry and merge strategy |
| `src/llm.rs` | LLM client abstraction |
| `src/writer.rs` | Note file writer |
| `src/prompt.rs` | LLM prompt construction |
| `migrations/0001_schema.sql` | Current database schema |

### 2.3 Current Database Schema (0001)

Four tables:

**`sources`** — ingested documents  
```sql
id, source_path, title, source_type, content_hash (SHA256, UNIQUE),
toc_hash, preprocessed_toc, toc_author, toc_generated_at, ingested_at, toc_text
```

**`notes`** — atomic entity records  
```sql
id (UUID), entity_type, name, match_key (UNIQUE: "entity_type:slug"),
file_path (NOT NULL — pointer to vault .md file),
summary_1 (one-line summary), summary_5 (paragraph summary),
merge_category, created_from (FK sources), source_count, created_at, updated_at
```

**`source_contributions`** — links sources to notes they contributed to  
```sql
id, source_id (FK), note_id (FK), toc_address, hint, contribution_type, payload, contributed_at
UNIQUE (source_id, note_id)
```

**`edges`** — knowledge graph relationships  
```sql
id, source_note_id (FK), target_note_id (FK), edge_type, why, from_source (FK),
weight (REAL, default 1.0), metadata (JSON), created_at
UNIQUE (source_note_id, target_note_id, edge_type, from_source)
```

### 2.4 Current NoteRecord Rust Struct

```rust
pub struct NoteRecord {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub match_key: String,
    pub file_path: String,         // currently NOT NULL — points to vault .md
    pub summary_1: Option<String>, // one-line summary
    pub summary_5: Option<String>, // paragraph summary
    pub merge_category: String,
    pub created_from: String,
    pub source_count: i64,
    pub created_at: String,
    pub updated_at: String,
}
```

### 2.5 Template System

19 entity template files in `source/templates/`. Each defines:
- `entity_type` — the canonical type string (e.g., `person`, `organization`, `note`, `chapter`, `area`)
- `template_class` — `identity` | `content_unit` | `source` | `utility`
- `merge_strategy` — `pure_atomic` | `container` | `source_bound`
- `identity_fields` — structured fields specific to this entity type (e.g., `contact_email`, `met_via` for person)
- `sources` — extraction hints per source type

### 2.6 Current MCP Tools (anansi_ingest, anansi_search, anansi_get, anansi_edges, anansi_relate)

The MCP server exposes five tools. `anansi_search` currently searches `name`, `summary_1`, and `summary_5` via SQL LIKE. `anansi_get` retrieves a note by UUID or match_key and reads the vault file if present. These will need field name updates after the schema migration.

### 2.7 Current Pipeline (the Old Way)

The existing `pipeline.rs` runs multiple LLM passes per source document:
- **Pass 1:** parse source file (frontmatter + body)
- **Pass 3:** per-entity LLM extraction — fields, summary_1, summary_5, tags
- **Pass 4:** LLM relationship extraction — generates edge records

This means N entities in a document = N+ LLM calls. A 67-note book ingestion would require dozens of LLM calls.

### 2.8 The web/ Folder (Original Design — Now Superseded)

The original design had three storage layers:
1. **vault/** — source documents (the originals)
2. **web/** — atomic note .md files generated by Anansi's background atomization process
3. **DB** — pointer records + summaries, edge table, vector index

The `file_path` in `notes` pointed to the `web/` file for that atomic note. Content retrieval required: DB query → file read from `web/`.

**This design is superseded.** See Section 8 for the replacement.

---

## 3. The Smart Brevity Insight

### 3.1 What Happened

James Pakele listened to the Smart Brevity audiobook (VandeHei, Allen, Schwartz — Workman Publishing, 2022) and recognized that its compression rules could transform the Anansi atomization pipeline from an expensive multi-LLM process into a two-LLM-call operation.

### 3.2 Smart Brevity in One Paragraph

Smart Brevity is a writing framework built on four elements called the **Core 4**:

1. **Muscular tease** — ≤6 words; active verb; the hook
2. **Lede (first sentence)** — ONE sentence; the single thing the reader must remember
3. **"Why it matters" axiom** — bolded signpost + 1–2 sentences of context; adds perspective, never repeats the lede
4. **"Go deeper"** — optional links/references for those who want more

Every piece of content — regardless of medium — is rewritten into this skeleton. Short, not shallow: every important fact is preserved; the packaging is stripped.

**The key rule:** brevity is confidence. Length is fear.

### 3.3 Why It Changes the Pipeline

The old pipeline generated `summary_1` and `summary_5` for each entity via individual LLM calls because the source content was prose — unstructured, requiring LLM comprehension to compress.

Smart Brevity atomization does that compression work **once, upfront**, for the entire document, in two LLM passes. The output is a structured block set where:
- Every note's **lede** is already written (the Smart Brevity first sentence)
- Every note's **"why it matters"** is already written (the axiom)
- Every note's **content** is already written (the full Smart Brevity block)

After those two passes, the remaining pipeline steps are pure code and database operations — no more LLM calls per note.

---

## 4. The PARA Framework

PARA (Tiago Forte, *Building a Second Brain*, 2022) is the classification framework Anansi uses to organize extracted knowledge. It defines four categories:

| Category | Definition | Example |
|---|---|---|
| **Projects** | Active outcomes with a deadline | "Launch Anansi v2 by Q2" |
| **Areas** | Ongoing responsibilities with a standard | "Written communication standards" |
| **Resources** | Entities and reference material | People, organizations, notes, concepts |
| **Concepts** | Tags and abstract ideas | `#smart-brevity`, `#brevity` |

In Anansi's PARA TOC format, these map to numbered sections:
- **Section 1:** Projects
- **Section 2:** Areas
- **Section 3:** Discussion (content sections from the source)
- **Section 4:** Resources (typed as person/organization/note)
- **Section 5:** Concepts (tags)

---

## 5. The Skills Architecture

### 5.1 Overview

Anansi's LLM-facing logic lives in **skills** — SKILL.md files that instruct Claude how to process content. They are packaged into **plugins** (`.plugin` zip archives) for distribution.

The relevant plugin is **`anansi-para`**, which contains four skills that form the processing pipeline. A fifth skill, **`smart-brevity`**, handles the compression pass.

### 5.2 The anansi-para Plugin

**Plugin location (when installed):** Claude local-agent-mode-sessions skills directory  
**Plugin package:** `smart-brevity.skill` (currently in `resources/` — needs proper plugin packaging)

Contains four skills:

#### 5.2.1 `para-extract`

**Input:** raw source document (any type: email, book, article, transcript, meeting notes)  
**Output:** structured PARA extraction — Projects, Areas, Resources (untyped), Concepts

The skill reads the document through the PARA lens: what active outcomes with deadlines does this surface? What ongoing responsibilities? What entities? What concepts/tags?

Resources at this stage carry a `vault_match_hint` field: `resource:[slug]`. These get typed in the next step.

**Key rule:** Projects require a deadline AND an outcome statement. Areas require a responsibility AND a standard. If neither is clear, it's a Resource.

#### 5.2.2 `resource-typer`

**Input:** para-extract output  
**Output:** same extraction with `resource:[slug]` placeholders replaced by typed keys: `person:[slug]`, `organization:[slug]`, `note:[slug]`

Driven by the entity templates in `references/templates/`. Reads each template's description and `identity_fields` to classify each resource.

**Important:** templates must be bundled inside the plugin — not stored separately. This is a known gap in the current implementation.

#### 5.2.3 `para-toc`

**Input:** source document + para-extract output + resource-typer output  
**Output:** a full typed PARA Table of Contents with decimal addressing

**Addressing rules (3-level hierarchy for books/hierarchical documents):**
```
3.x      — section header (Shape B, flat)
3.x.y    — chapter/subsection (Shape A, has lettered sub-entries)
3.x.y.a  — key point within a chapter (lettered, becomes ## in the block)
```

For flat documents (meeting notes, emails), 2 levels suffice:
```
3.x      — section
3.x.a    — key point
```

**Why 3 levels:** each chapter should be its own atomic note. Collapsing chapters as lettered sub-entries of their parent section means all chapters in a section would become a single vault note, defeating atomicity. Each `3.x.y` address = one vault note.

**Output format example:**
```
3.2 Part 1: What Is Smart Brevity? [section]

3.2.1 Short, Not Shallow [chapter]
  3.2.1.a Smart Brevity is not dumbing down — radical repackaging; "short, not shallow"
  3.2.1.b Universal scope: students, sales, leadership, nonprofits
  3.2.1.c Key rule: don't omit facts — oversimplifying violates the standard

4.1 Jim VandeHei [person]
  - Co-author; co-founder Axios and Politico; primary architect of Smart Brevity
  - vault_match_hint: person:jim-vandehei
```

#### 5.2.4 `para-pipeline`

Orchestrator skill. Runs para-extract → resource-typer → para-toc in sequence and returns the final TOC. Also handles template management (list, add, remove templates).

### 5.3 The `smart-brevity` Skill

**Input:** source document + PARA TOC (the output of para-pipeline)  
**Output:** one Smart Brevity block per decimal-addressed TOC entry, `---` separated

This is the **TOC Atomization Mode** of the smart-brevity skill. It generates the full atomized document in a single LLM pass.

#### 5.3.1 Two Block Shapes

**Shape A — Sectioned block** (for TOC entries with lettered sub-entries: chapters, sections with key points)

```
### [address] [title] [type-tag]
One lead sentence — the single most important thing this section establishes.

**Why it matters:** One sentence of context. Does not repeat the lead.

## [label from lettered entry a]
One tight sentence on this sub-topic.
- **Key term:** supporting fact
- **Key term:** supporting fact

## [label from lettered entry b]
One tight sentence.
- **Key term:** supporting fact

---
```

**Shape B — Capsule block** (for flat entries: section headers, persons, organizations, notes, areas)

```
### [address] [title] [type-tag]
One lead sentence — the most important fact about this entity.

**Why it matters:** One sentence on relevance. Omit if self-evident.

- **Key term:** fact
- **Key term:** fact

---
```

#### 5.3.2 Block Header Format (Patched)

The block header must include the title: `### [address] [title] [type-tag]`

Early versions of the skill used `### [address] [type-tag]` without the title. This was patched. The title is critical for the parser to generate meaningful note names without an additional LLM call.

#### 5.3.3 Output Format

```
<!-- smart-brevity atomization: [source title] | [N] blocks | [date] -->

### 2.1 Written Communication Standards [area]
...

---

### 3.1 Introduction: The Fog of Words [chapter]
...

---

<!-- concepts: #smart-brevity #brevity ... -->
```

The `---` separator is the **parse token**. Every block ends with `---`. The opening comment and closing concepts line are not blocks.

### 5.4 Worked Example: Smart Brevity Book

The book `smart-brevity.txt` (28,001 words) was processed through the full pipeline:

1. `para-pipeline smart-brevity.txt` → `smart-brevity-para-toc.md` (3-level, 67 addressable entries)
2. `/smart-brevity` TOC atomization → `smart-brevity-atomized.md` (67 blocks, `---` separated)

Both output files are in `resources/` and serve as reference implementations.

**Block count breakdown for the book:**
- 2 area blocks (Section 2)
- 28 discussion blocks (1 intro + 3 section headers + 24 chapters across Parts 1-3)
- 37 resource blocks (16 persons + 15 organizations + 6 notes)

---

## 6. The New Workflow (Authoritative)

```
SOURCE DOCUMENT
    │
    ▼
[atomize skill] — two-pass LLM pipeline (see plugins/anansi-para.plugin/skills/atomize/SKILL.md)
    Pass 1: para-pipeline → PARA TOC (para-extract → resource-typer → para-toc)
    Pass 2: smart-brevity TOC Atomization Mode
        → Input: source document + PARA TOC from Pass 1
        → Output: {source-slug}-atomized.md (N blocks, --- separated)
              Each block: ### {address} {title} [{type-tag}]
                          {lede}
                          **Why it matters:** {why}
                          {content body}
    │
    ▼
[CODE] Parse atomized.md
    → Split on `---` → one ParsedBlock per entry
    → Extract: address, title, type-tag → entity_type, match_key, lede, why, content
    → For each block: extract address, title, type-tag, entity_type, match_key
    │
    ▼
[DB] match_key lookup for each parsed block
    │
    ├── MATCH FOUND → Enhance existing note
    │       → Null-coalesce identity fields (fill empty fields from new source)
    │       → Append new bullet facts not already present (fuzzy dedup on bold key term)
    │       → If field conflict (both have value, values differ):
    │             · Append <!-- CONFLICT: field | new_value | timestamp --> to content
    │             · Set has_conflicts = 1, conflicts_updated_at = now()
    │             · Queue for notification
    │       → Insert source_contributions row (source_id, note_id, toc_address)
    │       → Increment source_count
    │       → Update updated_at
    │
    └── NO MATCH → Create new note
            → Insert into notes (lede, why, content, match_key, entity_type)
            → Insert source_contributions row
    │
    ▼
[CODE] Edge creation pass
    → For each discussion block (3.x.y): scan content for match_key patterns
    → Create edges: (chapter_note) → (person_note), (chapter_note) → (org_note), etc.
    → Insert into edges (source_note_id, target_note_id, edge_type, from_source, why)
    → Also create: (chapter) → (section) hierarchy edges, (resource) → (source) mention edges
    │
    ▼
[ASYNC] Embed lede
    → For each new/updated note: compute embedding from lede field
    → INSERT OR REPLACE INTO embeddings (note_id, model, dimensions, vector)
    → Runs as background queue, does not block ingest completion
    │
    ▼
[ASYNC] Conflict notification
    → Query: SELECT * FROM notes WHERE has_conflicts = 1 AND conflicts_updated_at > last_notified
    → Deliver via email/Slack/configured channel
    → (Channel implementation deferred — schema and query pattern are ready)
```

### 6.1 LLM Call Count Comparison

| Approach | LLM Calls for 67-note document |
|---|---|
| Old pipeline (per-entity passes) | 67–134+ calls |
| New pipeline | 3 calls (1a + 1b run in parallel; then call 2) |

### 6.2 What "Enhance" Means Precisely

When a match_key already exists in the DB, the enhance operation is:

1. **Identity field null-coalescing:** for each `identity_field` defined in the entity's template, if `existing[field]` is null/blank and `incoming[field]` has a value, write the new value. This handles cases like a person note gaining a `contact_email` or `phone` from a second source.

2. **Bullet deduplication:** split both existing and incoming content on bullet markers (`- **`). For each incoming bullet, check if a bullet with the same bold key term already exists in the content. If not, append it.

3. **Conflict detection:** if `existing[field]` is non-null AND `incoming[field]` is non-null AND they differ, do NOT overwrite. Append an inline conflict marker and set the flag.

4. **No LLM calls during enhance.** The lede is not regenerated on enhancement — the existing lede stands. An explicit `consolidate` command (future, user-initiated) could trigger a re-summarize pass when source_count exceeds a threshold.

---

## 7. Schema Migration 0002

Migration file: `source/migrations/0002_smart_brevity.sql`

### 7.1 Changes to `notes` Table

| Field | Before | After | Reason |
|---|---|---|---|
| `file_path` | `TEXT NOT NULL` | **REMOVED** | Notes store content inline. Source files are tracked in `sources.source_path` only. The file-pointer pattern is eliminated. |
| `summary_1` | `TEXT` | → renamed `lede` | Smart Brevity first sentence. Semantically precise: the single thing the reader must remember. No character limit (was implied ≤100 chars). |
| `summary_5` | `TEXT` | → renamed `why` | Smart Brevity "Why it matters" axiom. Shorter than summary_5 was (1-2 sentences vs 300-500 chars), but more useful for retrieval ranking. |
| `content` | (new) | `TEXT` | Full atomized block body (everything after the `why` line). Populated when a note is ingested from an `-atomized.md` file. NULL for notes inserted by the old pipeline. |
| `has_conflicts` | (new) | `INTEGER NOT NULL DEFAULT 0` | Flag set to 1 when a field conflict is written inline. Indexed for fast conflict queue queries. |
| `conflicts_updated_at` | (new) | `TEXT` (RFC3339 timestamp) | Timestamp of last conflict written. Enables "conflicts since last report" queries for notification batching. |

### 7.2 New `embeddings` Table

```sql
CREATE TABLE embeddings (
    id          TEXT    PRIMARY KEY,
    note_id     TEXT    NOT NULL,
    model       TEXT    NOT NULL,        -- e.g. "text-embedding-3-small"
    dimensions  INTEGER NOT NULL,        -- e.g. 1536
    vector      BLOB    NOT NULL,        -- raw f32 bytes, little-endian
    created_at  TEXT    NOT NULL,
    UNIQUE (note_id, model),             -- one embedding per note per model
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
);
```

**What to embed:** the `lede` field only. It is the most semantically dense single sentence in each note, consistent across all entity types, and small enough to embed cheaply. The `why` field is a secondary retrieval signal for re-ranking after vector recall — do not embed it separately.

**When to embed:** async, after note insert/update. Notes are immediately queryable via keyword search. Vector search improves as the embedding queue drains.

**Vector storage:** raw `f32` BLOB, little-endian. Store `dimensions` explicitly so the deserializer knows the array length without inspecting the model.

### 7.3 Inline Content

**Decision: store content inline in the DB. Eliminate web/ folder for new ingestion.**

**Single `content` field:** the atomize pipeline (PARA TOC pass → Smart Brevity formatting) produces three fields at once — `lede`, `why`, and `content` — for every note type. All three arrive together when a note is ingested from an `-atomized.md` file. There is no separate "SB pass" field; the atomize pipeline applies Smart Brevity formatting during Pass 2 and the result populates `content` directly.

**Why inline:** Smart Brevity blocks are 150-300 words each. At that size the file pointer pattern loses its main justification. Inline eliminates the DB query → file read two-step from `anansi_get`. `source_contributions.toc_address` already carries the full lineage.

**`file_path` is removed entirely from `notes`.** There is no dual-mode retrieval and no migration path for existing data — all prior note data is discarded when migration 0002 runs. Source files (the originals) are tracked in `sources.source_path`. Notes have no file path. `anansi_get` returns `lede`, `why`, and `content` inline; if `content IS NULL` (note not yet atomized by Build-09), it returns a structured warning.

### 7.4 Content Field Rules

| Field | Populated for | Source |
|---|---|---|
| `content` | All note types (when ingested from atomized file) | PARA TOC + Smart Brevity Pass 2 atomization |

`lede`, `why`, and `content` all arrive together from a single atomize pipeline run. Notes inserted by the old pipeline (Build-01 through Build-07) have `content IS NULL` — they were never atomized. The NULL state is the indicator that a note pre-dates the atomized ingest pipeline.

### 7.5 Updated NoteRecord Struct

```rust
pub struct NoteRecord {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub match_key: String,
    pub lede: Option<String>,
    pub why: Option<String>,
    pub content: Option<String>,
    pub has_conflicts: i64,
    pub conflicts_updated_at: Option<String>,
    pub merge_category: String,
    pub created_from: String,
    pub source_count: i64,
    pub created_at: String,
    pub updated_at: String,
}
```

> **Note:** `file_path` has been removed. Notes have no file pointer. Source files are in `sources.source_path`.

---

## 8. The Atomized File Format (Reference)

### 8.1 File Naming Convention

`{source-slug}-atomized.md`

Example: `smart-brevity-atomized.md`

This file is the **recomposed view** — the full document regenerated from all atomic notes. It was not in the original design; Smart Brevity makes it a natural byproduct of the LLM atomization pass. It can be regenerated at any time from the TOC + source.

### 8.2 Parsing Algorithm

```python
def parse_atomized(content: str) -> list[dict]:
    # Strip comment header and footer
    lines = content.strip().split('\n')
    
    blocks = []
    current = []
    
    for line in lines:
        if line.startswith('<!--'):
            continue  # skip comment lines
        if line.strip() == '---':
            if current:
                blocks.append(parse_block('\n'.join(current).strip()))
                current = []
        else:
            current.append(line)
    
    return blocks

def parse_block(block: str) -> dict:
    lines = block.split('\n')
    header = lines[0]  # e.g. "### 3.2.1 Short, Not Shallow [chapter]"
    
    # Extract address, title, type_tag from header
    # Pattern: ### {address} {title} [{type_tag}]
    import re
    m = re.match(r'^### (\S+)\s+(.+?)\s+\[(\w+)\]$', header)
    address = m.group(1)   # "3.2.1"
    title = m.group(2)     # "Short, Not Shallow"
    type_tag = m.group(3)  # "chapter"
    
    # First non-empty line after header = lede
    lede = next((l for l in lines[1:] if l.strip() and not l.startswith('**Why')), '')
    
    # "Why it matters" line
    why_line = next((l for l in lines if l.startswith('**Why it matters:**')), '')
    why = why_line.replace('**Why it matters:**', '').strip()
    
    # Full block content (everything after header)
    content = '\n'.join(lines[1:]).strip()
    
    # Compute match_key
    slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')
    match_key = f"{type_tag}:{slug}"
    
    return {
        'address': address,
        'title': title,
        'entity_type': type_tag,
        'match_key': match_key,
        'lede': lede,
        'why': why,
        'content': content,
    }
```

### 8.3 Type-Tag to Entity Type Mapping

| Type tag in block header | entity_type in DB |
|---|---|
| `[chapter]` | `chapter` |
| `[section]` | `section` |
| `[area]` | `area` |
| `[person]` | `person` |
| `[organization]` | `organization` |
| `[note]` | `note` |
| `[project]` | `project` |
| `[concept]` | `concept` |

---

## 9. Edge Creation Rules

After parsing all blocks into notes, run an edge creation pass:

### 9.1 Hierarchy Edges
For every chapter/section entry, create parent-child edges:
- `3.2.1` → `3.2` (child_of)
- `3.2` → `3.1` top-level container (part_of)

### 9.2 Mention Edges
For every discussion block (Section 3), scan the content for match_key patterns:
- If `person:ronald-yaros` appears in the content of `chapter:be-worthy`, create edge: `(chapter:be-worthy) → (person:ronald-yaros)` with edge_type `mentions`
- Same for organizations, notes

### 9.3 Source Attribution Edges
For every note: `(note) → (source)` with edge_type `derived_from`, populated via `source_contributions.source_id`

### 9.4 Concept Edges
The `<!-- concepts: #tag1 #tag2 -->` footer line produces concept nodes and edges: `(note) → (concept:tag)` for every concept in the document.

---

## 10. MCP Tool Updates Required

The following MCP tools in `src/mcp.rs` need updates after the schema migration:

### 10.1 `anansi_search`
Currently searches `summary_1` and `summary_5`. Update to search `lede` and `why`.

```sql
-- Before
WHERE name LIKE ? OR summary_1 LIKE ? OR summary_5 LIKE ?

-- After
WHERE name LIKE ? OR lede LIKE ? OR why LIKE ?
```

### 10.2 `anansi_get`
`file_path` is removed from notes. Update to:
1. Return `lede`, `why`, `content`, `has_conflicts`, and `conflicts_updated_at` inline — no file reading
2. If `content IS NULL` (note not yet atomized by Build-09), return metadata + `{"warning": "content_not_available", "message": "note has no inline content yet"}`

### 10.3 New tool: `anansi_ingest_atomized`
New tool for the Smart Brevity pipeline path. Accepts either:
- `atomized_path`: path to an `-atomized.md` file on disk
- `atomized_content` + `toc_path`: raw atomized content + the TOC file that generated it

Runs the parse → match_key lookup → enhance/new → edge creation pipeline.

### 10.4 New tool: `anansi_conflicts`
Returns all notes with `has_conflicts = 1`, ordered by `conflicts_updated_at DESC`.  
Optional `since` parameter (RFC3339) for "conflicts since last report" queries.

---

## 11. Conflict Inline Format

When a field conflict is detected during enhance, append to the note's `content` field:

```
<!-- CONFLICT: contact_email | new_value@source.com | 2026-04-30T14:22:00Z -->
```

Format: `<!-- CONFLICT: {field_name} | {new_value} | {rfc3339_timestamp} -->`

Multiple conflicts on the same note produce multiple comment lines. They are human-readable in the markdown and machine-parseable for the notification system.

The notification system (delivery channel TBD — email or Slack) should:
1. Query `SELECT * FROM notes WHERE has_conflicts = 1 AND conflicts_updated_at > ?`
2. For each flagged note: parse the `<!-- CONFLICT: -->` lines from content
3. Format a digest message with note name, field, existing value, new value, timestamp
4. After notification: update a `last_notified_at` marker (can be a single-row config table)

**A weekly digest of outstanding conflicts is the target UX.** The schema supports it with the current fields. Delivery implementation is deferred.

---

## 12. Implementation Task List

Tasks are ordered by dependency. Complete in sequence.

### Phase 1: Schema & Structs

- [ ] **Run migration 0002** (`source/migrations/0002_smart_brevity.sql` — already written)
- [ ] **Update `NoteRecord` struct** in `src/db.rs` — rename fields, add new fields, make `file_path` `Option<String>`
- [ ] **Update all DB queries** in `src/db.rs` — any query referencing `summary_1`, `summary_5`, or `file_path NOT NULL` needs updating
- [ ] **Update `anansi_search`** in `src/mcp.rs` — search `lede` and `why` (3 binds: name, lede, why)
- [ ] **Update `anansi_get`** in `src/mcp.rs` — inline content path + return new fields

### Phase 2: Atomized Ingest Pipeline

- [ ] **Write `src/atomized_parser.rs`** — parse atomized.md into `Vec<ParsedBlock>` structs using the algorithm in Section 8.2
- [ ] **Write `src/smart_ingest.rs`** — the new pipeline: parse → match_key lookup → enhance/new/conflict → edge creation
- [ ] **Write enhance logic** — null-coalesce identity fields from template, bullet dedup on bold key term, conflict detection and inline marker
- [ ] **Write edge creation pass** — hierarchy edges, mention edges, source attribution, concept edges
- [ ] **Add `anansi_ingest_atomized` MCP tool** — wires `smart_ingest.rs` to the MCP server

### Phase 3: Embeddings

- [ ] **Add embedding client** to `src/llm.rs` — embed API call (separate from completion calls; OpenAI-compatible endpoint)
- [ ] **Add `src/embedder.rs`** — async queue worker: takes note_id + lede, calls embed API, writes to `embeddings` table
- [ ] **Wire embedder** to the end of `smart_ingest.rs` ingest — push to queue after note insert/update
- [ ] **Update `anansi_search`** to support vector search path when query is semantic vs keyword

### Phase 4: Conflict Reporting

- [ ] **Add `anansi_conflicts` MCP tool** — query flagged notes, return digest
- [ ] **Add config option** for notification channel (email/Slack/webhook — implementation TBD)
- [ ] **Weekly digest scheduler** — cron or background task hitting `anansi_conflicts` and dispatching

### Phase 5: Template Bundling (Known Gap)

Templates are currently stored separately from the skills. They need to be bundled inside the `anansi-para` plugin alongside the skill files.

- [ ] **Move templates into plugin** — the `references/templates/` directory should be inside the `.plugin` archive
- [ ] **Update `resource-typer` skill** to load templates from plugin-relative path
- [ ] **Repackage `anansi-para` plugin**

---

## 13. Reference Files

All in `ai-anansi-v2/resources/`:

| File | Description |
|---|---|
| `smart-brevity.txt` | Full source text of the Smart Brevity book (28,001 words) |
| `smart-brevity-para-toc.md` | Reference PARA TOC — 3-level hierarchy, 67 entries |
| `smart-brevity-atomized.md` | Reference atomized output — 67 Smart Brevity blocks |
| `smart-brevity-SKILL.md` | Patched SKILL.md with `[address] [title] [type-tag]` header format |
| `smart-brevity.sb.md` | Reference direct Smart Brevity pass output — 25 chapter blocks, ~2,400 words (91% reduction from source) |
| `smart-brevity.skill` | Packaged skill file (zip archive) |

All in `ai-anansi-v2/source/`:

| File | Description |
|---|---|
| `migrations/0001_schema.sql` | Current schema |
| `migrations/0002_smart_brevity.sql` | Schema migration — already written, ready to run |
| `src/db.rs` | Structs and queries — needs NoteRecord update |
| `src/pipeline.rs` | Old multi-LLM pipeline — kept for non-Smart-Brevity ingest paths |
| `src/merger.rs` | Existing merge logic — reference for the enhance implementation |
| `src/mcp.rs` | MCP server — needs tool updates and two new tools |
| `templates/` | 19 entity template files |

---

## 14. Design Decisions Log

A record of key decisions made during the design session and the reasoning behind them.

**Q: Why is `file_path` removed from `notes` entirely rather than made nullable?**  
A: Smart Brevity blocks are 150-300 words. At that size, the file pointer pattern loses its main justification. Inline eliminates the DB→file two-step, simplifies retrieval, and `source_contributions.toc_address` already carries the lineage context the file was providing. Keeping `file_path` as `Option<String>` would have required dual-mode retrieval logic indefinitely — extra code, extra failure modes, extra cognitive overhead. Since there is no production data to preserve, removing it entirely is cleaner. Source files are tracked in `sources.source_path`; notes have no file pointer.

**Q: Why replace summary_5 with `why` (shorter, not longer)?**  
A: The "Why it matters" axiom (1-2 sentences) is more semantically loaded than a 5-line paragraph. It answers "why should I care about this note in context?" — a perspective signal, not a longer description. Better for retrieval ranking. The brevity is a feature.

**Q: Why embed `lede` only, not full content?**  
A: The lede is the most semantically dense single sentence, consistent across all 67+ note types, and small enough to embed cheaply. Full content at 200-300 words would work, but lede gives cleaner, more precise similarity — asking about the Core 4 shouldn't return a note about Politico's founding story just because they're in the same block.

**Q: Why 3-level TOC hierarchy for books?**  
A: Chapters are atomic concepts. Collapsing "Short, Not Shallow" + "Smart Brevity, Explained" + "The Road to Smart Brevity" + "Audience First" into a single "Part 1" note means all four concepts share one embedding, one vault note, and one retrieval result. The point of atomization is one concept per note. For books: section (3.x) → chapter (3.x.y) → key point (3.x.y.a).

**Q: Why only 3 LLM calls for the full pipeline (down from 67–134+)?**  
A: Call 1a (para-pipeline) and Call 1b (direct SB pass) run in parallel. Call 2 (TOC atomization) runs after 1a completes. Both bulk passes produce deterministic, parseable output — everything downstream is code. The old pipeline called the LLM per entity because prose content required per-entity comprehension. Smart Brevity does that comprehension work once, up front, for all entities simultaneously. The direct SB pass (1b) adds one call but captures editorial voice and punchy hooks that the systematic TOC extraction flattens out — worth the cost.

**Q: Should the enhance step ever call the LLM?**  
A: No, not automatically. Enhancement is template-aware field merge + bullet dedup — pure code. A future explicit `consolidate` command (user-initiated) could trigger a re-summarize pass for high source_count notes, but that is not part of the automatic pipeline.

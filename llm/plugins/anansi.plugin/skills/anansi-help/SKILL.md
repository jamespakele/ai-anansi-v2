---
name: anansi-help
description: >
  Returns complete internal documentation for the Anansi knowledge-graph
  engine. Covers architecture, data model, ingestion pipelines, MCP tools,
  LLM skills, template system, match_key semantics, merge strategies, and
  operational patterns. Read-only — does not modify anything. Use when the
  user (or another skill) asks "how does Anansi work", "what tools are
  available", "explain the pipeline", or any general orientation question
  about the system.
argument-hint: "[optional topic: tools | pipeline | templates | data-model | skills | rules | config]"
---

# anansi-help

One-stop reference for everything inside the Anansi v2 knowledge-graph engine.
Return this document's content (or the relevant section) whenever the user asks
how Anansi works, what tools exist, or how to accomplish a task.

If the user asks about a specific topic, jump to that section. If they ask a
general "how does Anansi work?" question, give them the **Overview** and
**Architecture** sections, then offer to drill into specifics.

---

## Overview

Anansi is an autonomous knowledge-graph engine. Drop any document into the
inbox; Anansi atomizes it into typed notes (people, projects, concepts, events,
etc.), graphs their relationships, and stores everything in PostgreSQL —
queryable over MCP from Claude Cowork, Claude Desktop, or any MCP client.

**Core loop:**
```
Source document → LLM pipeline → Typed atomic notes + edges → PostgreSQL → MCP API
```

**Key design principles:**
- **Atomic notes** — every note is one fact about one entity, never a dump
- **Templates** — entity types have structured schemas that control extraction
- **Match-key dedup** — entities merge across sources via deterministic keys
- **Graph edges** — relationships between notes are first-class citizens
- **Smart Brevity** — output follows Axios-style lede/why/content structure

---

## Architecture

### System components

| Component | What it does |
|-----------|--------------|
| **anansi2 binary** (Rust) | MCP server + inbox watcher + CLI |
| **PostgreSQL + pgvector** | Persistent storage, FTS, vector embeddings |
| **LLM skills** (markdown) | Prompt templates for pipeline stages |
| **MCP tools** (JSON-RPC) | API surface for Claude / any MCP client |
| **Templates** (markdown) | Entity type schemas that drive extraction |
| **%Rules** (markdown) | Architecture constraints the LLM must follow |

### Source modules

| Module | Purpose |
|--------|---------|
| `src/mcp.rs` | Axum HTTP server, JSON-RPC 2.0 dispatcher, all 19 MCP tools |
| `src/inbox.rs` | Background watcher, 3-stage pipeline orchestration |
| `src/llm.rs` | Multi-backend LLM client (gemini, openrouter, codex, ollama) |
| `src/atomized_ingest.rs` | Parses atomized markdown → PostgreSQL |
| `src/atomized_parser.rs` | Block parser for `<!-- anansi-atomize -->` format |
| `src/pipeline.rs` | Legacy 4-pass ingest pipeline |
| `src/template.rs` | Template parser and registry (disk + DB) |
| `src/merger.rs` | Note merge/conflict resolution logic |
| `src/db.rs` | PostgreSQL connection, migrations, match_key computation |
| `src/embed.rs` | Gemini embedding generation + pgvector storage |
| `src/export.rs` | BFS graph traversal, context export, Obsidian vault zip |
| `src/config.rs` | `anansi.toml` + environment variable loading |
| `src/vault.rs` | File-backed vault helpers |
| `src/writer.rs` | Markdown writer for pipeline output files |
| `src/prompt.rs` | Prompt assembly from skill files + templates + rules |
| `src/rules.rs` | Rule registry loader |
| `src/queue.rs` | Atomized-content queue watcher |

### Server startup flow

1. Load `anansi.toml` config
2. Connect to PostgreSQL, run migrations
3. Load templates from disk (`templates_dir`)
4. Seed any missing templates into DB as `anansi_config` notes
5. Load templates from DB (DB overrides disk — runtime source of truth)
6. Build LLM client (optional — not required for MCP-only operation)
7. Spawn queue watcher (polls `q-atomize/` for atomized content)
8. Spawn inbox watcher if enabled (polls `q-inbox/` for raw documents)
9. Start Axum HTTP server on configured port (default 3738)

---

## Data Model

### Database schema (PostgreSQL)

#### `notes` — the core entity table

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT PK | UUID |
| `entity_type` | TEXT | Type tag (person, project, topic, etc.) |
| `name` | TEXT | Display name |
| `match_key` | TEXT UNIQUE | Dedup key: `entity_type:slug` |
| `lede` | TEXT | The single most important fact (Smart Brevity) |
| `why` | TEXT | Why this entity matters — one sentence |
| `content` | TEXT | Detailed info, bullet points, structured data |
| `has_conflicts` | BIGINT | Conflict count for merge resolution |
| `merge_category` | TEXT | `entity`, `source_bound`, etc. |
| `created_from` | TEXT FK→sources | Source that created this note |
| `source_count` | BIGINT | Number of sources that mention this entity |
| `fts_vector` | tsvector | Auto-generated FTS column (name + lede + why + content) |

#### `sources` — ingested documents

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT PK | UUID |
| `source_path` | TEXT | Original file path |
| `title` | TEXT | Document title |
| `source_type` | TEXT | meeting_summary, email_thread, etc. |
| `content_hash` | TEXT UNIQUE | SHA for dedup |
| `toc_text` | TEXT | PARA TOC from Pass 1 |

#### `edges` — relationships between notes

| Column | Type | Description |
|--------|------|-------------|
| `source_note_id` | TEXT FK→notes | From note |
| `target_note_id` | TEXT FK→notes | To note |
| `edge_type` | TEXT | Relationship label (mentions, belongs_to, etc.) |
| `why` | TEXT | Reason for the relationship |
| `from_source` | TEXT FK→sources | Which source created this edge |
| `weight` | FLOAT | Edge weight (default 1.0) |
| UNIQUE | | (source_note_id, target_note_id, edge_type, from_source) |

#### `source_contributions` — tracks which sources contributed to which notes

Links sources to the notes they contributed to, with TOC address and contribution type.

#### `note_embeddings` — vector embeddings for semantic search

| Column | Type | Description |
|--------|------|-------------|
| `note_id` | TEXT FK→notes | Note this embedding belongs to |
| `model` | TEXT | Embedding model name |
| `embedding` | vector(768) | pgvector embedding |
| PK | | (note_id, model) |

HNSW index for fast approximate nearest-neighbour cosine search.

### match_key semantics

The `match_key` is how Anansi deduplicates entities across sources. Computed as:

```
match_key = entity_type + ":" + slugify(name)
```

Slugification: lowercase → replace non-alphanumeric with spaces → collapse whitespace → join with hyphens.

Examples:
- `"Ian Kitajima"` + `"person"` → `person:ian-kitajima`
- `"PICHTR"` + `"organization"` → `organization:pichtr`
- `"O'Brien & Co."` + `"organization"` → `organization:o-brien-co`

**Important:** colons in the name are normalized to hyphens. For `anansi_config`
notes that need literal colons in the key (e.g., `anansi_config:template:place`),
pass `match_key` explicitly to `anansi_capture`.

### Archive convention

Archiving prepends `archive-` to entity_type (e.g., `person` → `archive-person`).
Archived notes are excluded from search/filter by default. Pass
`include_archived: true` or filter by `entity_type: "archive-<type>"` to find them.

---

## Ingestion Pipelines

### Pipeline 1: Inbox (full LLM pipeline)

For raw documents that need decomposition and extraction.

```
/data/inbox/<file>.md
       │
       ▼  (inbox watcher — automatic)
  Stage 1 ─────────────────────────────── (parallel LLM calls)
    ├── para-projects-areas    → projects-areas-toc.md + projects-areas-typed.md
    └── para-resource-entities → resources-toc.md     + resources-typed.md
       │
       ▼
  Stage 2: sb-atomize          → <slug>-atomized.md (Smart Brevity blocks)
       │
       ▼
  Stage 3: ingest_atomized     → PostgreSQL (notes, edges, sources)
```

Intermediate files checkpointed in `/data/archive/<slug>-<timestamp>/`.

### Pipeline 2: Atomized ingest (zero LLM)

For pre-atomized content (output of sb-atomize or hand-crafted blocks).

```
anansi_ingest_atomized(content) → atomized_parser → PostgreSQL
```

No LLM calls. Parses `<!-- anansi-atomize: ... -->` blocks directly.

### Pipeline 3: Quick capture (zero LLM)

For single entities — person, event, topic, etc.

```
anansi_capture(entity_type, name, lede, ...) → upsert by match_key → PostgreSQL
```

No LLM calls. Direct upsert via match_key.

---

## MCP Tools Reference

### Read tools (safe, no side effects)

| Tool | Purpose | Key args |
|------|---------|----------|
| `anansi_search` | Full-text keyword search (tsvector) | `query`, `limit`, `operator` (or/and) |
| `anansi_search_semantic` | Vector similarity search (pgvector) | `query`, `limit` |
| `anansi_get` | Fetch one note by `id` or `match_key` | `id` or `match_key` |
| `anansi_filter` | List notes by entity_type and/or date range | `entity_type`, `after`, `before`, `limit` |
| `anansi_edges` | BFS graph traversal from a note | `id`, `depth` (1–5) |
| `anansi_export_context` | BFS → flat markdown for LLM consumption | `id`, `depth` |
| `anansi_export_vault` | BFS → Obsidian-compatible zip download | `id`, `depth` |
| `anansi_list_entity_types` | List all registered templates | `template_class`, `verbose` |
| `anansi_get_upload_url` | Returns HTTP upload URLs + curl commands | `local_path` |

### Write tools (modify database)

| Tool | Purpose | Key args |
|------|---------|----------|
| `anansi_capture` | Upsert a single note by match_key | `entity_type`, `name`, `lede`, `match_key` (anansi_config only) |
| `anansi_update_note` | Patch an existing note by UUID | `note_id`, then any field to change |
| `anansi_relate` | Create an edge between two notes | `source_id`, `target_id`, `edge_type` |
| `anansi_ingest_atomized` | Ingest pre-atomized blocks | `content` or `path` |
| `anansi_ingest_file` | Queue raw doc for full pipeline | `content`, `filename` |
| `anansi_delete_note` | Hard-delete a note + its edges | `note_id` |
| `anansi_archive_note` | Soft-archive/restore a note | `note_id`, `restore` |
| `anansi_purge` | Delete everything from one source | `source_id` |
| `anansi_embed` | Generate vector embeddings | `note_id` or `batch: true` |
| `anansi_reload_templates` | Hot-reload template registry from DB | (no args) |

---

## LLM Skills Reference

### Ingestion skills (write path)

| Skill | When to use | Route |
|-------|-------------|-------|
| **anansi-remember** | "Remember this" — single entry point for all writes | Routes to anansi-atom, sb-atomize, or ingest_atomized |
| **anansi-atom** | Capture a single named entity | Template lookup → Smart Brevity → `anansi_capture` |
| **anansi-ingest-file** | Ingest a raw document | Upload to inbox or `anansi_ingest_file` |
| **anansi-update** | Edit an existing note in place | Fetch → show → `anansi_capture` (upsert) |

### Retrieval skills (read path)

| Skill | When to use | Tools used |
|-------|-------------|------------|
| **anansi-recall** | "What do you know about X?" | wiki-first → `anansi_get`, `anansi_search`, `anansi_edges` |
| **anansi-recompose** | Reconstruct a full document from atoms | Outline walk → entity lookup (wiki-first) → markdown assembly |
| **anansi-digest** | *(deprecated — use anansi-recompose)* | — |

**Wiki-first reads.** The read skills consult the local **LLM-wiki** — a markdown mirror of the vault at `/home/pakele/llm-wiki` (the PARA-parent, Obsidian-readable, matching `[wiki] dir`) — *before* the database, then fall back to the MCP tools on a miss. This absorbs read traffic from Postgres and works offline. The wiki is a **cache, never authoritative**: it may lag the DB by a crawl interval and omits size-evicted cold notes, and **mutations** (delete/archive/update/relate/purge) always resolve their target via the database, never the wiki. Full procedure: `references/wiki-first.md`. Rebuild the wiki anytime from Postgres with `anansi2 rebuild-wiki`.

### Management skills

| Skill | When to use |
|-------|-------------|
| **anansi-delete** | Remove or archive a note |
| **anansi-purge** | Remove an entire source import |
| **anansi-new-entity-type** | Create a new entity type template |
| **scaffold-template** | Generate a template file from a description |

### Pipeline skills (server-side, used by inbox watcher)

| Skill | Pipeline stage | What it does |
|-------|---------------|--------------|
| **para-process** | Stage 1 orchestrator | Runs para-projects-areas + para-resource-entities in parallel |
| **para-projects-areas** | Stage 1a | Extracts Projects and Areas from source |
| **para-resource-entities** | Stage 1b | Extracts Resources and typed entities from source |
| **sb-atomize** | Stage 2 | Merges Stage 1 outputs into Smart Brevity atomic blocks |

---

## Template System

### What templates are

Templates define entity types — the schema that controls how entities are
extracted, stored, and merged. Each template has:

1. **YAML frontmatter** — entity_type, template_class, merge_strategy, identity_fields, source hints
2. **`%%` field blocks** — per-field extraction instructions for the LLM
3. **Body template** — markdown with `{{field_name}}` placeholders

### Template classes

| Class | What it represents | Examples |
|-------|--------------------|----------|
| `identity` | Durable real-world thing. Persists across documents. | person, organization, project, area |
| `content_unit` | Bounded unit of meaning within a source. Floor type. | topic_discussion, email_exchange, youtube_chapter |
| `source` | A whole document being atomized. Decomposes into content_units. | meeting_summary, email_thread, research_paper |
| `utility` | Flexible glue that doesn't fit above. | event, task, context, note, outline |

### Merge strategies

| Strategy | Behavior | Used by |
|----------|----------|---------|
| `pure_atomic` | No body accumulation. Each capture overwrites. | Most identity types |
| `container` | Has roster sections that accumulate rows additively. | organization, project (have member/reference rosters) |
| `source_bound` | Tied to one source. No cross-source merging. | All content_units and sources |
| `title_author` | *(reserved)* | Books (dedupe by title+author) |

### Template storage hierarchy

1. **Disk** — `llm/plugins/anansi.plugin/references/templates/` (compiled into binary via `include_str!`)
2. **DB** — `anansi_config` notes with `match_key: anansi_config:template:<entity_type>`
3. **Runtime** — DB overrides disk. `anansi_reload_templates` refreshes from DB without restart.

### Creating new entity types

Use the **anansi-new-entity-type** skill. It:
1. Checks existing types for overlap
2. Walks through guided questions (class, merge strategy, fields, etc.)
3. Generates the template
4. Stores it via `anansi_capture` with `match_key: anansi_config:template:<type>`
5. Hot-reloads via `anansi_reload_templates`

---

## %Rules (Architecture Constraints)

The `%Rules/` directory contains architecture rules the LLM must follow during
pipeline processing. They are loaded into the rule registry at startup.

| Rule | What it enforces |
|------|-----------------|
| **%Atomicity** | One note = one entity. No multi-entity dumps. Fields must be atomic. |
| **%Downstream-Flow** | How data flows through pipeline stages. Each stage's contract. |
| **%Merge-Strategy** | How notes merge when the same entity appears in multiple sources. |
| **%Template-Schema** | The YAML frontmatter + field block + body template format. |

---

## Configuration

### anansi.toml

```toml
[paths]
web_dir       = "anansi/web"
rules_dir     = "anansi/%Rules"
templates_dir = "anansi/templates"

[llm]
backend    = "gemini"      # gemini | openrouter | codex | ollama
url        = "..."         # ollama URL (ignored for gemini/openrouter)
model      = "..."         # model name
timeout_s  = 600

[server]
mcp_port  = 3738
host      = "0.0.0.0"
read_only = false
# api_key = "..."          # optional auth

[inbox]
enabled            = true
watch_dir          = "/data/inbox"
archive_dir        = "/data/archive"
skills_dir         = "/data/skills"
poll_interval_secs = 30
```

### LLM backends

| Backend | Auth | Notes |
|---------|------|-------|
| **gemini** | CLI OAuth or API key | Recommended. Uses Gemini CLI first, falls back to REST. |
| **openrouter** | API key | Any frontier model via one key. |
| **codex** | Device-flow OAuth | OpenAI Codex CLI. |
| **ollama** | None | Local models. Minimum qwen2.5:14b recommended. |

### Environment variables

- `DATABASE_URL` — PostgreSQL connection string
- `ANANSI_GEMINI_API_KEY` — Gemini REST fallback
- `ANANSI_OPENROUTER_API_KEY` — OpenRouter auth
- `ANANSI_API_KEY` — optional MCP endpoint auth
- `ANANSI_CODEX_CLI_PATH` — override Codex CLI binary path

---

## Common Patterns

### "Remember something" flow

User says "remember that John Doe works at Acme Corp" →
1. Invoke **anansi-remember** skill
2. Routes to **anansi-atom** (single entity detected)
3. anansi-atom identifies entity_type `person`, reads person template
4. Formats with Smart Brevity (lede, why, content)
5. Calls `anansi_capture` → upserts by `match_key: person:john-doe`
6. Confirms to user

### "What do you know about X?" flow

User says "what do you know about Acme Corp?" →
1. Invoke **anansi-recall** skill
2. Infers `match_key: organization:acme-corp` → tries `anansi_get`
3. If not found, falls back to `anansi_search` with "Acme Corp"
4. Presents results with lede, why, content
5. Optionally follows edges with `anansi_edges` for related entities

### "Ingest this document" flow

User pastes meeting notes →
1. Invoke **anansi-remember** skill
2. Routes to multi-entity path (B)
3. Runs **para-process** (parallel: projects-areas + resource-entities)
4. Runs **sb-atomize** on the combined output
5. Calls `anansi_ingest_atomized` → notes + edges into PostgreSQL
6. Reports counts: notes created, merged, edges created

### "Delete/archive something" flow

User says "archive the note about old project" →
1. Invoke **anansi-delete** skill
2. Searches for the note, confirms with user
3. Calls `anansi_archive_note` (soft delete — reversible)
4. Entity type becomes `archive-project`, excluded from normal queries

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| "note not found" on capture | match_key mismatch | Check entity_type + name slugification. Use `anansi_search` to find existing. |
| Template not recognized | Not in registry | Call `anansi_list_entity_types`. If missing, use anansi-new-entity-type to create. |
| Inbox not processing | Watcher disabled | Check `[inbox] enabled = true` in anansi.toml. Check logs for errors. |
| Semantic search empty | No embeddings | Run `anansi_embed` with `batch: true` to generate embeddings. |
| Duplicate notes | Different entity_type for same concept | match_key includes entity_type — `person:john-doe` ≠ `note:john-doe`. Pick one type. |
| Template changes not taking effect | DB is source of truth | Call `anansi_reload_templates` after editing DB templates. Disk templates only used at startup. |

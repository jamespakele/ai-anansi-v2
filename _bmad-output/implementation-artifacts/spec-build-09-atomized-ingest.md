---
title: 'Anansi v2 — Build 09: Atomized File Ingest Parser'
type: 'feature'
created: '2026-04-30'
status: 'review'
baseline_commit: ''
context:
  - docs/HANDOFF-anansi-smart-brevity.md
  - plugins/anansi-para.plugin/skills/atomize/SKILL.md
  - plugins/anansi-para.plugin/skills/smart-brevity/SKILL.md
  - plugins/anansi-para.plugin/skills/para-toc/SKILL.md
---

## Intent

**Problem:** After Build-08 the notes table is empty. The old LLM pipeline (`pipeline.rs` Pass 1–4) is superseded. The new ingest path is:

1. Claude runs the `atomize` skill on a source document — **Pass 1** produces a typed PARA TOC (`PARA_TOC`); **Pass 2** produces Smart Brevity blocks (`TOC_BLOCKS`, saved as `{slug}-atomized.md` or held in memory)
2. Claude calls `anansi_ingest_atomized` passing both outputs — the atomized blocks and optionally the PARA TOC text
3. Rust parses each block and saves it with all three fields (`lede`, `why`, `content`) populated; the PARA TOC is stored on the source record and the outline note

**Approach:** (1) `src/atomized_parser.rs` — pure string parser: splits atomized text on `---` separators, extracts `address`/`title`/`entity_type`/`lede`/`why`/`content` from each block, derives a minimal address skeleton (`raw_toc`) from the block headers. (2) `src/atomized_ingest.rs` — ingest pipeline: register source (`toc_text = para_toc`), create one **outline note** (content = `para_toc` when available, else `raw_toc`), loop over parsed blocks with a **section dispatch** (`match section_num { 1 => project, 2 => area, 3 => discussion, 4 => resource, _ => skip }`), create `source_contributions`, hierarchy edges, and outline→block edges. (3) `anansi_ingest_atomized` MCP tool — accepts atomized blocks as text or file path, plus optional `para_toc` and `source_path`.

> **No LLM calls.** The `atomize` skill (running in Claude) does all LLM work. Build-09 is mechanical: parse → dispatch → insert.

> **No new migrations.** The Build-08 schema is sufficient.

## The Full Flow

**`toc` variant** — produced by the `atomize` skill (recommended; includes `para_toc`):
```
User: "atomize book.md"
  └─ Claude: runs atomize skill
       └─ Pass 1: para-pipeline → PARA_TOC (typed TOC with addresses + bullet facts)
       └─ Pass 2: smart-brevity TOC Atomization Mode → TOC_BLOCKS
       └─ Saves TOC_BLOCKS as {slug}-atomized.md (header: <!-- anansi-atomize: ... -->)
            variant = "toc"

User: "ingest into anansi"
  └─ Claude calls:
       anansi_ingest_atomized {
         content: <TOC_BLOCKS text>,
         para_toc: <PARA_TOC text>,       ← Pass 1 output; stored on source + outline note
         source_path: "path/to/book.md"   ← for attribution
       }
```

**`sb` variant** — produced by calling the `smart-brevity` skill standalone (no `atomize` wrapper):
```
User: "run smart-brevity on book.md"
  └─ Claude: runs smart-brevity skill directly → SB_BLOCKS
       └─ Saves as {slug}-sb.md (header: <!-- smart-brevity atomization: ... -->)
            variant = "sb"

User: "ingest into anansi"
  └─ Claude calls:
       anansi_ingest_atomized {
         content: <SB_BLOCKS text>,       ← no para_toc available in this path
         source_path: "path/to/book.md"
       }
       └─ raw_toc derived from block headers (para_toc not available)
```

**Both paths converge at Rust:**
```
  └─ Rust: parse blocks → Vec<ParsedBlock>
       └─ Register source (toc_text = PARA_TOC or raw_toc)
       └─ Create outline note (content = PARA_TOC or raw_toc; always overwritten on re-ingest)
       └─ Loop blocks with section dispatch:
            1.x → Project handler
            2.x → Area handler
            3.x → Discussion handler (match_key gets ":toc" or ":sb" suffix)
            4.x → Resource handler
       └─ Create edges (block→outline "part_of", parent→child "contains", declared edges)
       └─ Return IngestAtomizedReport { status: "ok", ... }
```

## Boundaries & Constraints

**Always:**
- `src/atomized_parser.rs` is pure — no `async`, no DB, no file I/O; only `&str → Result<AtomizedFile>`
- All three fields (`lede`, `why`, `content`) populated for every successfully parsed block; `why` may be `None` only when the block genuinely omits `**Why it matters:**`
- Block loop uses `match section_number { 1 | 2 | 3 | 4 => ..., _ => skip }` where `section_number` is the integer prefix of the address before the first `.`
- **Section 3 (discussion) match_keys** use `format!("{}:{}", db::match_key(title, entity_type), variant)` — e.g. `"book-chapter:basb-chapter-13:toc"`
- **Sections 1, 2, 4 (entity) match_keys** use base `db::match_key(title, entity_type)` only — NO variant suffix; edge references in `### Edges` use base match_keys and must resolve against entity notes
- The `### Edges` section in a block is stripped from `content` entirely; edge lines become edge table records
- One **outline note** per source: `entity_type = "outline"`, content = `para_toc` if provided, else `raw_toc` derived from block headers
- `para_toc` is stored in both `sources.toc_text` (source-level access) and the outline note's `content` (graph-level access)
- Re-ingest of identical atomized content (same SHA-256) returns early without duplicating data
- Block→outline edges: `edge_type = "part_of"`, each block note points BACK to the outline note (`source_note_id = block_id`, `target_note_id = outline_id`)
- Hierarchy edges: `edge_type = "contains"`, direct parent→child address only (`source_note_id = parent_id`, `target_note_id = child_id`)
- `cargo check` and `cargo test` must pass

**Ask First:**
- Any change to `merger.rs`, `pipeline.rs`, or existing DB migrations
- Adding Section 5 concept tags as notes (they're stored in `AtomizedFile.concepts` but don't become note records in this build)

**Never:**
- Call an LLM during atomized ingest
- Leave `lede` NULL for a parsed block; `content` is `None` only when the block has no body after lede/why (stripped-down entity blocks) — the `ParsedBlock.content` field is `Option<String>` and `None` is valid for empty content
- Include the `### Edges` section or any edge lines in the stored `content` field
- Add a variant suffix to entity note match_keys (sections 1, 2, 4) — only section 3 discussion notes get the suffix
- Alpha sub-addresses as separate notes — they appear as content within their parent block, not as block entries

## PARA Section Roles

| Section | Role | Typical entity_types | match_key form | `### Edges` |
|---|---|---|---|---|
| `1.x` | Projects — active outcomes | `project` | base `project:slug` | yes (owner, vendor, references) |
| `2.x` | Areas — ongoing responsibilities | `area` | base `area:slug` | yes (owned_by, supported_by) |
| `3.x` | Discussion — content/article | `book-chapter`, `architecture`, `meeting-topic-discussion`, etc. | **variant** `type:slug:toc\|sb` | yes (under_project, references, etc.) |
| `4.x` | Resources — typed entities | `person`, `organization`, `note`, `book` | base `type:slug` | orgs/notes/projects yes; **`[person]` never** |
| `5.x` | Concepts — tags only | (none) | (no blocks) | — |

**`[person]` blocks never have `### Edges`.** All person relationships surface through the blocks of the things they are connected to (a project block owns a person; an org block has an affiliate).

The section number is the canonical dispatch key — the `entity_type` tag supplies the fine-grained type within each section.

## Atomized File Format (Parser Contract)

### File header (two accepted forms)
```
<!-- anansi-atomize: {source title} | {N} toc-blocks | {date} -->
<!-- smart-brevity atomization: {source title} | {N} blocks | {date} -->
```

### Block structure — separated by `---` alone on its own line
```
### {address} {title} [{type-tag}]
{lede — first non-empty line after header; required; NOT included in content}

**Why it matters:** {why text — rest of this line; strip prefix; optional; NOT included in content}

{content — lines AFTER lede/why and BEFORE ### Edges; None if empty}

### Edges
- {relationship_verb}: {entity_type}:{slug}
- {relationship_verb}: {entity_type}:{slug}

---
```

**`### Edges` parsing rules:**
- `### Edges` heading (exactly `^\s*###\s+Edges\s*$`) marks the boundary — everything from this line onward (until `---`) is the edges section; this section is stripped from `content` entirely
- `[person]` blocks never have `### Edges` — the parser must not fail if absent
- Omitted entirely when no relationships are inferable — the parser handles its absence gracefully
- Edge line format: `- {edge_type}: {target_match_key}` where `target_match_key` is `entity_type:slug` (base, no variant)
- Empty `### Edges` sections do not appear (self-check in the skill) but parser handles gracefully if they do

### File footer
```
<!-- concepts: #tag1 #tag2 -->
```
Concept tags are stored in `AtomizedFile.concepts`; they do not become note records in this build.

### `raw_toc` skeleton (derived from block headers when `para_toc` not supplied)
```
1.1 Project Alpha [project]
2.1 Area One [area]
3.1 Introduction [book-chapter]
3.1.1 Background [book-chapter]
4.1 Ian Kitajima [person]
```
One line per decimal-addressed block in address order. `raw_toc` covers block headers only — alpha sub-entries (`3.1.a`) appear within block `content` prose and are NOT extracted into `raw_toc`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Fresh ingest, `para_toc` provided | Empty notes table | Source `toc_text` = PARA_TOC; outline note `content` = PARA_TOC; all blocks saved; edges created | — |
| Fresh ingest, no `para_toc` | Empty notes table | Source `toc_text` = derived `raw_toc`; outline note `content` = derived `raw_toc` | — |
| Re-ingest same atomized content | `content_hash` already in `sources` | `Ok(IngestAtomizedReport { status: "already_ingested", source_id: "...", ..Default::default() })` → `{"status":"already_ingested","source_id":"..."}` — no writes | — |
| Re-ingest revised content | Different atomized output, same source | New source record; COALESCE fills NULLs only on block notes; outline note `content` always overwritten with new `toc_text`; `source_count + 1` | — |
| Block missing `**Why it matters:**` | Entity block omits why | `why = None`; lede and content parsed normally | — |
| Block omits lede; first body line is `**Why it matters:**` | Malformed block structure | `parse_block` returns `Err`; `blocks_skipped += 1`; valid blocks still ingested | — |
| Block at address `5.x` | Concepts bucket (should not appear as block) | Match arm `_` → skip; `blocks_skipped += 1` | — |
| Block in section 1–4 with unknown address depth | `3.1.1.2` — nested child | Parsed and inserted normally; hierarchy edge created if parent `3.1.1` also present | — |
| `content` and `path` both provided | Both present in tool call | `content` takes precedence; `path` stored as `source_path` if `source_path` not explicitly provided | — |
| Neither `content` nor `path` | Tool call missing both | JSON-RPC error: `"must provide either content or path"` | — |
| Malformed block header | No `### … [type]` on first non-empty line | Skip; `blocks_skipped += 1`; valid blocks still ingested | — |

## Code Map

- `src/lib.rs` — add `pub mod atomized_parser;` and `pub mod atomized_ingest;`
- `src/atomized_parser.rs` — **[NEW]** Pure parser: `AtomizedFile`, `ParsedBlock`, `parse_atomized_file`
- `src/atomized_ingest.rs` — **[NEW]** Ingest with section dispatch: `IngestAtomizedReport`, `ingest_atomized`, `section_num`, `is_direct_child`
- `src/mcp.rs` — add `anansi_ingest_atomized` to tool list + `tool_ingest_atomized` handler
- `Cargo.toml` — add `sha2 = "0.10"` if absent; verify `uuid` with v4 feature
- `src/db.rs` — no changes; all required functions already exist

## Tasks & Acceptance

**Execution:**

- [x] `src/atomized_parser.rs` — **[NEW]**

  ```rust
  pub struct ParsedEdge {
      pub edge_type: String,   // "owner", "references", "managed_by", "under_project", etc.
      pub target_mk: String,   // base match_key: "person:richmond", "organization:hipa"
  }

  pub struct ParsedBlock {
      pub address: String,
      pub title: String,
      pub entity_type: String,
      pub lede: String,
      pub why: Option<String>,     // "**Why it matters:** " prefix stripped
      pub content: Option<String>, // everything before ### Edges; None if empty
      pub edges: Vec<ParsedEdge>,  // empty if no ### Edges section present
  }

  pub struct AtomizedFile {
      pub source_title: String,
      pub date: String,
      pub variant: String,         // "toc" (anansi-atomize header) or "sb" (smart-brevity header)
      pub blocks: Vec<ParsedBlock>,
      pub skipped_count: usize,
      pub concepts: Vec<String>,   // ["#closed-learning-loop", ...]
      pub raw_toc: String,         // address-skeleton: one line per decimal-addressed block in order
  }

  pub fn parse_atomized_file(text: &str) -> Result<AtomizedFile>
  ```

  `parse_atomized_file` algorithm:
  1. Find the first `<!-- ... -->` line. Match either header form and set `variant`:
     - `<!-- anansi-atomize: ... -->` → `variant = "toc"`
     - `<!-- smart-brevity atomization: ... -->` → `variant = "sb"`
     Split the matched content on `|`. Return `Err` if fewer than three `|`-separated fields are present. `source_title` = first field (trimmed). `date` = third field (trimmed). (Middle field is the block count — parsed for reference, not stored.) Return `Err` if neither header form matches.
  2. Find the `<!-- concepts: ... -->` line. Parse `#word` tokens from it into `concepts`. Remove this line from the text before block splitting.
  3. Split remaining text on lines matching `^\s*---\s*$`. Discard segments that are entirely whitespace.
  4. For each segment, call `parse_block(segment: &str) -> Result<ParsedBlock>`:
     - Match the first non-empty line against `^\s*###\s+(?P<address>\d[\d.]*)\s+(?P<title>.+?)\s+\[(?P<entity_type>[A-Za-z][A-Za-z0-9_-]*)\]\s*$`. Return `Err` if no match.
     - **Split body from edges**: find the first line matching `^\s*###\s+Edges\s*$`. Everything before it = `body_lines`. Everything after it = `edge_lines`. If absent, all lines are `body_lines` and `edge_lines` is empty.
     - Find `lede` in `body_lines`: first non-empty line after the header. Return `Err` if absent. If the lede candidate starts with `**Why it matters:**`, return `Err` (block is missing its lede — malformed).
     - Scan `body_lines` for one starting with `**Why it matters:**`. If found: `why = Some(rest_of_line.trim().to_string())`. Lines after the why line → `content_lines`.
     - If no why line: `why = None`. All lines after the lede line → `content_lines`.
     - `content = Some(content_lines.join("\n").trim().to_string())`, or `None` if empty.
     - Parse `edge_lines`: for each line matching `^\s*-\s*(?P<edge_type>[a-z_]+):\s*(?P<target_mk>[a-z][a-z0-9_-]*:[a-z0-9_-]+)\s*$`, push `ParsedEdge { edge_type, target_mk }`. Skip malformed edge lines silently.
  5. Collect all `Ok(block)` into `blocks`; count `Err` results as `skipped_count`.
  6. Build `raw_toc`: for each successfully parsed block in address order, emit `"{address} {title} [{entity_type}]"`. One entry per block; alpha sub-entries within block content are not included.

  Unit tests in `#[cfg(test)]`:
  - `parse_anansi_atomize_header` — `<!-- anansi-atomize: ... -->` header form accepted
  - `parse_smart_brevity_header` — `<!-- smart-brevity atomization: ... -->` header form accepted
  - `parse_shape_a_block` — `## Sub-heading` sections land in `content`; `why` stripped of prefix
  - `parse_shape_b_block` — bullet list in `content`
  - `parse_omits_why` — no `**Why it matters:**` line → `why == None`
  - `parse_empty_content` — only lede + why → `content == None`
  - `parse_concepts` — `<!-- concepts: #a #b -->` → `vec!["#a", "#b"]`
  - `parse_skips_malformed` — block without `### … [type]` → `skipped_count == 1`; other blocks parsed
  - `raw_toc_format` — `raw_toc` has one line per block in address order
  - `variant_toc` — `<!-- anansi-atomize: ... -->` header → `variant == "toc"`
  - `variant_sb` — `<!-- smart-brevity atomization: ... -->` header → `variant == "sb"`
  - `parse_edges_section` — block with `### Edges` section: edges extracted into `ParsedBlock.edges`; those lines absent from `content`
  - `parse_no_edges_section` — `[person]` block with no `### Edges`; `block.edges` is empty; `content` unaffected
  - `parse_edges_content_boundary` — content before `### Edges` is preserved; `### Edges` line and below excluded from `content`

- [x] `src/atomized_ingest.rs` — **[NEW]**

  ```rust
  pub struct IngestAtomizedReport {
      pub status: String,           // "ok" on success; "already_ingested" on hash match (all other fields zero/empty)
      pub source_id: String,
      pub outline_note_id: String,
      pub source_title: String,
      pub blocks_parsed: usize,
      pub blocks_skipped: usize,
      pub notes_created: usize,
      pub notes_enhanced: usize,    // note already existed before this ingest (find_note_by_match_key returned Some)
      pub edges_created: usize,
      pub unresolved_edges: usize,  // declared edges whose target_mk had no matching note
      pub concepts: Vec<String>,
  }

  pub async fn ingest_atomized(
      pool: &DbPool,
      content: &str,              // atomized blocks text
      para_toc: Option<&str>,     // PARA TOC from Pass 1; stored on source + outline note
      source_path: Option<&str>,  // original source file path for attribution
  ) -> Result<IngestAtomizedReport>
  ```

  Helper functions (private):
  ```rust
  fn section_num(address: &str) -> Option<u32>  // "3.1.1" → Some(3); "" → None
  fn is_direct_child(child: &str, parent: &str) -> bool {
      // Use safe slicing via .get() to avoid UTF-8 boundary panics.
      // A trailing-dot address like "1.2." is rejected by the suffix .contains('.') check.
      match child.get(parent.len()..) {
          Some(suffix) if suffix.starts_with('.') => {
              match child.get(parent.len() + 1..) {
                  Some(rest) => child.starts_with(parent) && !rest.contains('.') && !rest.is_empty(),
                  None => false,
              }
          }
          _ => false,
      }
  }
  ```

  Ingest steps:
  1. **Hash** `content` bytes with SHA-256 → lowercase hex string (`content_hash`).
  2. **Dedup**: `find_source_by_content_hash(pool, &content_hash).await?`. If `Some(s)`: return `Ok(IngestAtomizedReport { status: "already_ingested".into(), source_id: s.id, ..Default::default() })`.
  3. **Parse**: `parse_atomized_file(content)?`.
  4. **TOC text**: `let toc_text = para_toc.unwrap_or(&parsed.raw_toc).to_string();`
  5. **Register source**: `let fallback_path = format!("atomized:{}", db::match_key(&parsed.source_title, "source")); SourceRecord { id: Uuid::new_v4()..., source_path: source_path.unwrap_or(&fallback_path), title: Some(parsed.source_title.clone()), source_type: "atomized", content_hash, toc_hash: None, preprocessed_toc: 0, toc_author: None, toc_generated_at: None, ingested_at: now_rfc3339(), toc_text: Some(toc_text.clone()) }`. Call `insert_source(pool, &src).await?`.
  6. **Create outline note**:
     - `let outline_mk = match_key(&parsed.source_title, "outline")`
     - Build `NoteRecord`: `entity_type = "outline"`, `name = parsed.source_title`, `match_key = outline_mk`, `lede = Some(format!("Outline for {}.", parsed.source_title))`, `why = None`, `content = Some(toc_text.clone())`, other fields as standard
     - `insert_note(pool, &outline_rec).await?`
     - **Always overwrite outline content**: run `sqlx::query!("UPDATE notes SET content = ?1 WHERE match_key = ?2", toc_text, outline_mk).execute(pool).await?` — ensures re-ingest of revised content updates the structural skeleton even when COALESCE would otherwise skip a non-NULL value.
     - **Resolve actual outline_note_id**: re-fetch via `find_note_by_match_key(pool, &outline_mk).await?`; propagate `Err` if `None` (should be unreachable after insert).
  7. **Block loop with section dispatch**:

     ```rust
     let mut address_index: HashMap<String, String> = HashMap::new(); // address → note_id

     for block in &parsed.blocks {
         if address_index.contains_key(&block.address) {
             // Duplicate address in file — log and skip to avoid silent edge loss
             eprintln!("anansi_ingest: duplicate address {} in file; skipping second occurrence", block.address);
             report.blocks_skipped += 1;
             continue;
         }
         let note_id = match section_num(&block.address) {
             Some(1) => process_block(pool, block, &src, &parsed.variant, &mut report).await?,  // Project
             Some(2) => process_block(pool, block, &src, &parsed.variant, &mut report).await?,  // Area
             Some(3) => process_block(pool, block, &src, &parsed.variant, &mut report).await?,  // Discussion
             Some(4) => process_block(pool, block, &src, &parsed.variant, &mut report).await?,  // Resource
             _ => { report.blocks_skipped += 1; continue; }
         };
         address_index.insert(block.address.clone(), note_id);
     }
     ```

     `process_block(pool, block, src, variant, report)` (private async fn) for a single block:
     a. Derive match_key based on section:
        ```rust
        let base_mk = match_key(&block.title, &block.entity_type);
        let note_mk = match section_num(&block.address) {
            Some(3) => format!("{}:{}", base_mk, variant), // "architecture:core-architecture:toc"
            _       => base_mk,                            // "person:richmond", "project:concon-501c4-formation"
        };
        ```
        Section 3 discussion notes carry the variant suffix; entity notes (1/2/4) do not — edge references target entity notes by base match_key and must resolve.
     b. `let existing = find_note_by_match_key(pool, &note_mk).await?;`
     c. `let note_id = existing.as_ref().map(|n| n.id.clone()).unwrap_or_else(|| Uuid::new_v4().to_string());`
     d. Build `NoteRecord`: all three fields set from block; `created_from = src.id.clone()`.
     e. `insert_note(pool, &rec).await?`
     f. If `existing.is_some()`: `report.notes_enhanced += 1`; else `report.notes_created += 1`.
     g. `insert_contribution(pool, &SourceContributionRecord { source_id: src.id, note_id: note_id.clone(), toc_address: Some(block.address.clone()), contribution_type: "atomized", ... }).await.ok();` — `.ok()` silences UNIQUE constraint on duplicate `(source_id, note_id)`.
     h. Return `note_id`.

  **After the block loop: all notes are now in the DB (or already existed). Edge resolution runs second because edge targets from later blocks (e.g., `person:richmond` at 4.1) must exist before edges from earlier blocks (e.g., project 1.1 → `person:richmond`) can be created.**

  8. **Block → outline edges** (`part_of`): For each `(address, note_id)` in `address_index`, edge FROM block BACK TO outline:
     - `EdgeRecord { source_note_id: note_id, target_note_id: outline_note_id, edge_type: "part_of", from_source: src.id, weight: 1.0, ... }`
     - `if insert_edge_if_not_exists(pool, &e).await? { report.edges_created += 1; }`
     - To reconstruct: `SELECT source_note_id FROM edges WHERE target_note_id = ? AND edge_type = 'part_of'` → fetch each block note ordered by `source_contributions.toc_address`.

  9. **Hierarchy edges** (`contains`): For each pair `(addr_a, id_a)`, `(addr_b, id_b)` in `address_index` where `is_direct_child(addr_b, addr_a)`:
     - `EdgeRecord { source_note_id: id_a, target_note_id: id_b, edge_type: "contains", from_source: src.id, ... }`
     - `if insert_edge_if_not_exists(pool, &e).await? { report.edges_created += 1; }`

  10. **Block-declared edges** (`### Edges` section): For each block in `parsed.blocks`, for each `ParsedEdge` in `block.edges`:
     - `let source_id = address_index.get(&block.address)` — must exist (block was just inserted)
     - `let target_note = find_note_by_match_key(pool, &edge.target_mk).await?`
     - If `target_note.is_none()`: skip; add to `report.unresolved_edges` count (target not yet in DB — may be in a different source file)
     - Build `EdgeRecord { source_note_id: source_id, target_note_id: target_note.id, edge_type: edge.edge_type.clone(), from_source: src.id, weight: 1.0, why: None, metadata: None, ... }`
     - `if insert_edge_if_not_exists(pool, &e).await? { report.edges_created += 1; }`

  11. **Return** `IngestAtomizedReport`.

- [x] `src/mcp.rs` — Add to tools JSON array and add handler:

  Tool descriptor:
  ```json
  {
    "name": "anansi_ingest_atomized",
    "description": "Ingest an atomized block set from the atomize or smart-brevity skill. Pass the atomized content directly (preferred — avoids disk write) or a file path. Optionally pass the PARA TOC from Pass 1 to store as the outline note content and source record. Creates notes with lede/why/content, an outline note for document recomposition, and hierarchy edges.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "content": {
          "type": "string",
          "description": "The full atomized blocks text (<!-- anansi-atomize: ... --> through <!-- concepts: ... -->). Preferred — pass the in-memory Pass 2 output directly."
        },
        "path": {
          "type": "string",
          "description": "Absolute path to a saved atomized .md file. Used only if content is not provided."
        },
        "para_toc": {
          "type": "string",
          "description": "Optional. The typed PARA TOC output from Pass 1 of the atomize skill. Stored as the source record's toc_text and as the outline note's content — enables full document recomposition."
        },
        "source_path": {
          "type": "string",
          "description": "Optional. Path to the original source document being atomized (for source record attribution)."
        }
      }
    }
  }
  ```

  Handler:
  - Resolve `content_str`: if `content` param present use it; else if `path` present:
    - Canonicalize the path and verify it starts with `ctx.anansi_root`; return JSON-RPC error -32602 if it escapes root.
    - `tokio::fs::read_to_string(&canonical_path).await?`
    - else return error `"must provide either content or path"`.
  - `para_toc` = `params.get("para_toc").and_then(|v| v.as_str())`
  - `source_path` = `params.get("source_path").and_then(|v| v.as_str())`
  - Call `atomized_ingest::ingest_atomized(&ctx.db, &content_str, para_toc, source_path).await`
  - `Ok(report)` where `report.status == "already_ingested"` → return `{"status":"already_ingested","source_id":"<report.source_id>"}`
  - `Ok(report)` otherwise → return JSON with all report fields
  - `Err` → JSON-RPC internal error

- [x] `Cargo.toml` — `grep sha2 Cargo.toml`; add `sha2 = "0.10"` if absent. `grep -A3 '"uuid"' Cargo.toml`; verify `features = ["v4"]`.

- [x] `src/lib.rs` — add `pub mod atomized_parser;` and `pub mod atomized_ingest;`.

**Acceptance Criteria:**

- `cargo check` exits 0
- `cargo test` — all existing tests pass; all new parser tests pass
- Given two-block atomized file (`3.2 Core Architecture [architecture]` + `4.16 Hermes Agent [note]`) with `para_toc` provided: source record `toc_text` = the para_toc text; outline note `content` = the para_toc text; `notes_created = 2`; `edges_created = 2` (two `part_of` edges FROM each block TO the outline; no `contains` since neither address is parent of the other)
- Given same file without `para_toc`: source `toc_text` = derived `raw_toc`; outline note `content` = derived `raw_toc`
- Given file with `3.1 Discussion [book-section]` + `3.1.1 Chapter One [book-chapter]`: `edges_created` includes one `"contains"` edge from `3.1` to `3.1.1`; both also have `part_of` edges to outline
- Given `anansi_get` on an ingested block note: non-null `"lede"`, `"content"`; no `"warning"` key
- Given `anansi_get` on the outline note: `"entity_type": "outline"`, `"content"` contains the TOC structure
- Given re-ingest of same atomized content: MCP response `{"status":"already_ingested","source_id":"..."}` — no new rows; `ingest_atomized` returned `Ok` (not `Err`)
- Given `anansi_ingest_atomized` with `content` param: works without any disk file
- Given two blocks with the same address in one file: second block `blocks_skipped += 1`; first block fully ingested with all edges
- Given a block with address `5.1` (concepts bucket): `blocks_skipped += 1`; other blocks ingested
- Given a malformed block (no `### … [type]` header): `blocks_skipped += 1`; remaining blocks ingested
- Given a `toc`-variant ingest followed by an `sb`-variant ingest of the same chapter title (3.x block): two separate note records — `"book-chapter:chapter-13:toc"` and `"book-chapter:chapter-13:sb"` — no collision
- Given a project block with `### Edges` referencing `person:richmond` and `person:richmond` exists in the same file as a 4.x block: the edge record is created; `person:richmond` stored under base match_key (no variant suffix)
- Given the c4-structure-matrix.md example file: `anansi_ingest_atomized` creates all expected edge records (e.g., `1.1 ConCon` → `owner: person:richmond`); no entity note has a variant suffix in its match_key
- Given a block whose `### Edges` references a `note:slug` not present in the file: `unresolved_edges += 1`; no error; all other notes and edges created normally
- Given a `[person]` block: `block.edges` is empty; `content` field contains only the contact bullets (no `### Edges` line present)

## Design Notes

**Why section dispatch by leading number:** The four PARA sections have distinct roles. Dispatching on `section_num(address)` makes the code self-documenting and provides clear extension points — future builds can add section-specific logic (e.g., project blocks link to task edges; resource blocks trigger entity deduplication checks) without touching a monolithic handler.

**Why `para_toc` is the skeleton and why it lives in the outline note's `content`:** The PARA TOC from Pass 1 IS the skeleton — it contains the full address structure plus the bullet-fact summaries from the source document before Smart Brevity compression. Storing it in the outline note's `content` makes it the structural anchor of the knowledge graph: open the outline note, read the full map, follow `part_of` edges (incoming) to navigate to any constituent note. The `raw_toc` fallback (addresses + titles derived from block headers only) is minimal and lossy — usable when Pass 1 output wasn't captured, but the full PARA TOC is strongly preferred.

**Why block→outline edges (not outline→block):** Each atomic note carries a `part_of` edge pointing back to the skeleton/outline it came from. This lets the outline act as a reconstruction anchor: query all edges where `target_note_id = outline_id AND edge_type = 'part_of'` to get all parts, then order by `source_contributions.toc_address`. The edge runs FROM the block TO the outline because the block is the unit of knowledge that "belongs to" the outline — the outline is the structure it participates in. `contains` (parent→child address hierarchy) is semantically distinct and preserved separately.

**Why `match_key(title, entity_type)` for blocks:** Same derivation as old pipeline. Entity notes (person, org, area, project) deduplicate across multiple atomized sources. Discussion notes (chapter, section) use their title + type as identity, which works for uniquely-named sections.

**Why `insert_contribution` with `.ok()` for UNIQUE suppression:** One source can contribute to a note at most once (UNIQUE(source_id, note_id)). If the same entity appears at two TOC addresses in the same file (e.g., a person mentioned in both Section 3 discussion and Section 4 resources), the second contribution silently fails. The note is still upserted correctly; only the contribution tracking is coarse. Pre-existing design constraint.

**Variant suffix scoped to section 3 only:** Discussion notes (3.x) carry the variant suffix — `"book-chapter:basb-chapter-13:toc"` vs `"book-chapter:basb-chapter-13:sb"` — so both pipeline versions coexist without collision. Entity notes (1.x, 2.x, 4.x) use the base match_key only (`"person:richmond"`, `"project:concon-501c4-formation"`). This is required for edge resolution: `### Edges` references use base match_keys, and the lookup `find_note_by_match_key(pool, "person:richmond")` must find the note. If entity notes carried variant suffixes, all declared edges would be unresolvable.

**Why `### Edges` content is stripped from the `content` field:** The `content` field is prose — what the note IS about. Edge declarations are graph structure — how it RELATES to other notes. Mixing them pollutes the content with machine-readable syntax that would confuse full-text search and display rendering. The edge records in the `edges` table are the right home for this data.

**`[person]` blocks never have `### Edges`:** Person identity is atomic. A person's relationships surface through the convergence blocks of the things they are connected to — the project that owns them, the org they are affiliated with, the note they presented. This keeps person records clean and avoids the fan-out problem of trying to enumerate all relationships from a person's perspective at ingest time.

**Outline note as a full-document embedding anchor (theory):** The outline note's `content` = the PARA TOC, which contains the full structural map plus extracted bullet-facts for the entire source document. When Phase 3 embeddings are implemented, embedding the outline note's content may yield a vector that represents the entire source document's knowledge topology — a single query vector that spans all topics rather than any single atomic note's scope. Each block note is then a refinement of that vector space. This is theoretical but worth preserving: the outline note is the natural candidate for a document-level embedding, while block notes provide concept-level embeddings.

**`anansi_search` still does not search `content`:** Intentional. Full-text search over prose blocks requires FTS5. Deferred to a future build.

## Verification

```bash
cargo check
cargo test

# After ingest:
sqlite3 <vault>/anansi.db \
  "SELECT entity_type, name, lede IS NOT NULL, content IS NOT NULL FROM notes;"
# Expect: outline + block notes all have lede/content set

sqlite3 <vault>/anansi.db \
  "SELECT edge_type, count(*) FROM edges GROUP BY edge_type;"
# Expect: part_of = N blocks, contains = parent-child pairs

sqlite3 <vault>/anansi.db \
  "SELECT toc_text IS NOT NULL FROM sources WHERE source_type='atomized';"
# Expect: 1 (toc_text populated)
```

### Review Findings

#### Decision Needed

- [x] [Review][Decision] **Dedup-as-error anti-pattern** → resolved: add `status: String` to `IngestAtomizedReport`; return `Ok(report)` with `status = "already_ingested"` and all counts zero; MCP handler checks `report.status` instead of matching an error prefix. (See patch P14.)
- [x] [Review][Decision] **Outline note `content` silently stale on re-ingest** → resolved: after `insert_note` for the outline note, always run an explicit UPDATE to overwrite `content` with `toc_text` regardless of prior value. (See patch P15.)

#### Patch

- [x] [Review][Patch] **`process_block` missing `variant` parameter** — fixed: signature updated to `process_block(pool, block, src, variant, report)`; variant passed from caller as `&parsed.variant` [atomized_ingest.rs tasks, process_block step a]
- [x] [Review][Patch] **`is_direct_child` unsafe string slices** — fixed: rewritten using `.get()` with `Option` returns; trailing-dot and empty-suffix cases handled [atomized_ingest.rs tasks, is_direct_child]
- [x] [Review][Patch] **`content` nullability self-contradictory** — fixed: "Never" rule rewritten to "`content` is None only when block has no body after lede/why" [Boundaries & Constraints Never section]
- [x] [Review][Patch] **`raw_toc` alpha sub-entry extraction unspecified** — fixed: alpha sub-entries removed from `raw_toc` spec; `raw_toc` covers decimal-addressed block headers only [atomized_parser.rs task step 6]
- [x] [Review][Patch] **Content field description implies lede is in `content`** — fixed: block structure diagram updated to show lede/why are NOT included in `content` [Atomized File Format, block structure]
- [x] [Review][Patch] **`source_path` fallback slug undefined** — fixed: fallback defined as `format!("atomized:{}", db::match_key(&parsed.source_title, "source"))` [atomized_ingest.rs tasks step 5]
- [x] [Review][Patch] **`notes_enhanced` increment condition undefined** — fixed: `notes_enhanced` increments when `find_note_by_match_key` returns `Some`; `notes_created` when `None`; comment added to struct field [atomized_ingest.rs tasks]
- [x] [Review][Patch] **Header pipe count not validated** — fixed: step 1 requires exactly two `|` separators; returns `Err` if fewer than three pipe-separated fields [atomized_parser.rs task step 1]
- [x] [Review][Patch] **Lede consumed as Why line if lede absent** — fixed: after extracting lede candidate, if it starts with `**Why it matters:**` return `Err` (malformed block) [atomized_parser.rs parse_block lede step]
- [x] [Review][Patch] **Duplicate block addresses silently overwrite `address_index`** — fixed: duplicate check before process_block; second occurrence logged and skipped [atomized_ingest.rs tasks step 7]
- [x] [Review][Patch] **MCP `path` parameter has no path-traversal guard** — fixed: canonicalize + anansi_root prefix check; return -32602 if it escapes [mcp.rs handler]
- [x] [Review][Patch] **`sb` variant flow never described in Full Flow** — fixed: Full Flow section now shows both `toc` and `sb` paths [The Full Flow section]
- [x] [Review][Patch] **`atomize/SKILL.md` references nonexistent `content_toc` field** — fixed: parser field routing updated to correctly describe `lede`, `why`, `content` fields [plugins/anansi-para.plugin/skills/atomize/SKILL.md]
- [x] [Review][Patch] **`IngestAtomizedReport` needs `status` field; dedup returns `Ok`** — fixed: `status: String` added to struct; dedup returns `Ok` with `status = "already_ingested"`; MCP handler routes on `report.status` [atomized_ingest.rs tasks, mcp.rs handler]
- [x] [Review][Patch] **Outline note `content` must always overwrite on ingest** — fixed: explicit UPDATE after `insert_note` always writes current `toc_text` to outline note [atomized_ingest.rs tasks step 6]

#### Deferred

- [x] [Review][Defer] `toc_hash: None` hardcoded without explanation [ingest step 5] — deferred, pre-existing schema field; low impact
- [x] [Review][Defer] `inputSchema` missing `required`/`oneOf` constraint [mcp.rs tool descriptor] — deferred, runtime validation handles this
- [x] [Review][Defer] Outline note match_key collision for same-slug source titles [ingest step 6] — deferred, pre-existing COALESCE design constraint
- [x] [Review][Defer] YAML frontmatter causes false `---` split [atomized_parser.rs step 3] — deferred, atomize skill never emits frontmatter
- [x] [Review][Defer] `insert_contribution` `.ok()` silences errors with no report visibility [process_block step g] — deferred, explicitly acknowledged design tradeoff in spec
- [x] [Review][Defer] Build-08 migration `0002_smart_brevity.sql` not verified as committed — deferred, verify via `ls migrations/`; db.rs reflects correct schema
- [x] [Review][Defer] `source_count` increments once per block per note (multi-address same file) — deferred, by design with COALESCE
- [x] [Review][Defer] COALESCE prevents updating entity block content after first non-NULL ingest — deferred, intentional design
- [x] [Review][Defer] Edge slug regex accepts underscores; `match_key` never produces them — deferred, overpermissive but not incorrect
- [x] [Review][Defer] No dedicated unit test for SHA-256 dedup — deferred, integration ACs cover this
- [x] [Review][Defer] Edge entity_type regex rejects uppercase — deferred, atomize skill always emits lowercase entity_types
- [x] [Review][Defer] `insert_contribution` UNIQUE double-insert theoretical — deferred, unreachable in normal flow

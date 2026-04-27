---
name: remember
description: >-
  Store a source document in the anansi knowledge vault in one motion.
  Claude performs Pass 1 (TOC extraction with entity typing); the local
  anansi daemon handles Pass 3 (entity expansion) and Pass 4 (relationship
  extraction) via its configured LLM backend. Use when the user says
  "remember this", "save this to anansi", or "ingest this document".
argument-hint: "[vault_root]"
---

Take a source document — meeting notes, an email thread, a research summary, any prose — and store it in the anansi knowledge vault in a single motion. You do the structural thinking (Pass 1 TOC with entity typing); the local anansi daemon handles entity expansion (Pass 3) and relationship extraction (Pass 4) via its configured LLM backend (Ollama, Gemini, or OpenRouter).

---

## Prerequisites

The anansi MCP server must be running and registered:
- **Claude Desktop**: `anansi2 --root <vault_root> serve` — connects directly to `localhost:3738`
- **ChatGPT Developer Mode / Codex Desktop**: same command, but expose it via a Cloudflare Tunnel (`cloudflared tunnel --url http://localhost:3738`) and register the tunnel HTTPS URL as the MCP endpoint

If the server is unavailable, the skill degrades gracefully (Step 9).

---

## TOC Text Format

Each leaf on its own line:

```
{address} {Name} [{entity_type}] | hint:{one sentence description}
```

- `address`: dot-separated integers, max depth 6 (e.g. `1`, `1.2`, `1.2.3`)
- `Name`: the entity's canonical display name, case-preserved
- `[entity_type]`: must be one of the types read from the templates folder in Step 2 — never use `?`
- `| hint:...`: concise one-sentence description of this entity in the context of this document

Example:
```
1 Digital Futures Workshop [event] | hint:Strategic planning session hosted by PICHTR in April 2026.
1.1 James Pakele [person] | hint:Founder of Pakele.ai, leading AI strategy discussion.
1.2 PICHTR [organization] | hint:Pacific International Center for High Technology Research, event host.
1.3 Anansi [topic] | hint:AI-powered knowledge decomposition pipeline under development.
1.4 Sovereign AI [concept] | hint:Framework for locally-controlled AI infrastructure without cloud dependency.
```

---

## Steps

### Step 1 — Identify context

Determine:
- **Vault root**: the directory where `anansi/anansi.toml` lives. Ask the user if not clear from conversation.
- **Source filename**: the original filename (e.g. `pakele-ai-kickoff-2026-04-24.md`). Derive from the document title or ask.

### Step 2 — Read available entity types

List the files in `<vault_root>/anansi/templates/`. Each `.md` filename stem is a valid entity type (e.g. `person.md` → `person`). Exclude `outline` and `container` — those are internal types used by the daemon, not TOC leaf types.

Use **only** types from this list when building the TOC. The `note` type is always the fallback for anything that doesn't fit a more specific type.

### Step 3 — Read source content

Get the full source text via:
- The conversation (user pasted it), or
- Read tool if the user provided a file path

### Step 4 — Identify all entities

Read the full document carefully. Extract every meaningful named entity. Include entities mentioned briefly, not just those with dedicated sections. Assign a type from the list you read in Step 2. Build a hierarchical address structure reflecting the document's natural structure.

Aim for completeness over brevity: a missed entity cannot be linked in the graph.

### Step 5 — Build the anansi_toc block

Write out every leaf in the TOC text format. Requirements:
- Every leaf gets a type from the Step 2 list — never leave `[?]`
- Addresses must be unique and valid (`\d+(\.\d+)*`)
- Depth ≤ 6
- `hint:` is required on every line

### Step 6 — Splice frontmatter

Build the augmented source content in memory:

**If the source already has YAML frontmatter** (starts with `---`):
Insert the `anansi_toc` key inside the existing frontmatter block:
```yaml
---
existing_key: existing_value
anansi_toc: |
  1 Entity Name [type] | hint:...
  1.1 Child Name [type] | hint:...
---
```

**If no frontmatter exists**, prepend a new block:
```yaml
---
anansi_toc: |
  1 Entity Name [type] | hint:...
---
```

Do not write to disk yet. Hold the augmented content in memory.

### Step 7 — Call anansi_ingest

Call the `anansi_ingest` MCP tool with:
```json
{
  "content": "<full augmented source content>",
  "filename": "<original filename>"
}
```

This single call:
- Writes the source file to `<vault_root>/<filename>`
- Validates the `anansi_toc` frontmatter and skips Pass 1
- Runs Pass 3 (entity expansion) and Pass 4 (relationship extraction) on the local daemon
- Writes `<vault_root>/anansi/web/<source-slug>.outline.md` and typed leaf files

### Step 8 — Report result

The `anansi_ingest` response will be one of:

**Queued (large document):** `{ "status": "queued", "source_path": "...", "message": "..." }`
- Report: "Ingest started. The daemon is processing `<filename>` in the background — Pass 3 expansion for large TOCs takes a few minutes. Use `anansi_search` to verify results."
- The source file is already written to vault root at the path shown in `source_path`.

**Legacy sync success:** `{ "source_id": "...", "outline_note_id": "...", ... }`
- Report: Source ID, count of leaves, and outline location `anansi/web/<source-slug>.outline.md`.

### Step 9 — Fallback (server unreachable)

If the MCP tool call fails or the server is not registered:

1. Write the augmented content (with `anansi_toc` frontmatter) to disk:
   - Path: `<vault_root>/<filename>`
   - Use the Write tool

2. Print the manual ingest command:
   ```
   anansi2 --root <vault_root> ingest <vault_root>/<filename>
   ```

3. Explain that the server needs to be running and registered before calling this skill again.

# Skill: anansi-remember

Take a source document — meeting notes, an email thread, a research summary, any prose — and store it in the anansi knowledge vault in a single motion. You do the structural thinking (Pass 1 TOC with entity typing); the local anansi daemon handles entity expansion (Pass 3) and relationship extraction (Pass 4) on the local GPU.

---

## Prerequisites

The anansi MCP server must be running and registered:
- **Claude Desktop**: `anansi2 --root <vault_root> serve` — connects directly to `localhost:3738`
- **ChatGPT Developer Mode / Codex Desktop**: same command, but expose it via a Cloudflare Tunnel (`cloudflared tunnel --url http://localhost:3738`) and register the tunnel HTTPS URL as the MCP endpoint

If the server is unavailable, the skill degrades gracefully (Step 8).

---

## Entity Type Taxonomy

Assign exactly one type to every leaf in the TOC:

| Type | Use for |
|------|---------|
| `person` | Named individual human |
| `organization` | Company, institution, team, government body, non-profit |
| `topic` | Technology, discipline, theme, subject area |
| `concept` | Abstract idea, framework, methodology, principle |
| `event` | Conference, meeting, workshop, dated occurrence |
| `project` | Named initiative, product, programme |
| `note` | Anything that doesn't fit the above — default fallback |

---

## TOC Text Format

Each leaf on its own line:

```
{address} {Name} [{entity_type}] | hint:{one sentence description}
```

- `address`: dot-separated integers, max depth 6 (e.g. `1`, `1.2`, `1.2.3`)
- `Name`: the entity's canonical display name, case-preserved
- `[entity_type]`: one of the types above — never use `?`
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

### Step 2 — Read source content

Get the full source text via:
- The conversation (user pasted it), or
- Read tool if the user provided a file path

### Step 3 — Identify all entities

Read the full document carefully. Extract every meaningful named entity — people, organizations, topics, concepts, events, projects. Include entities mentioned briefly, not just those with dedicated sections. Assign a type from the taxonomy above. Build a hierarchical address structure reflecting the document's natural structure.

Aim for completeness over brevity: a missed entity cannot be linked in the graph.

### Step 4 — Build the anansi_toc block

Write out every leaf in the TOC text format. Requirements:
- Every leaf gets a type — never leave `[?]`
- Addresses must be unique and valid (`\d+(\.\d+)*`)
- Depth ≤ 6
- `summary:` is required on every line

### Step 5 — Splice frontmatter

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

### Step 6 — Call anansi_ingest

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

### Step 7 — Report success

On a successful response, report:
- `source_id` returned
- `outline_note_id` returned
- Count of leaves submitted
- Location of the outline file: `anansi/web/<source-slug>.outline.md`

Example:
> Stored. Source ID: `a1b2c3...`, outline at `anansi/web/pakele-ai-kickoff-2026-04-24.outline.md`. 7 entities queued for Pass 3 expansion.

### Step 8 — Fallback (server unreachable)

If the MCP tool call fails or the server is not registered:

1. Write the augmented content (with `anansi_toc` frontmatter) to disk:
   - Path: `<vault_root>/<filename>`
   - Use the Write tool

2. Print the manual ingest command:
   ```
   anansi2 --root <vault_root> ingest <vault_root>/<filename>
   ```

3. Explain that the server needs to be running and registered before calling this skill again.

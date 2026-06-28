---
name: wiki-ingest-atomized
description: >
  Client-side dual-write to the local ~/llm-wiki folder. Takes the same
  atomized content that goes to anansi_ingest_atomized and writes each
  entity block as a markdown file in ~/llm-wiki/, matching the server-side
  wiki format (## Connections, frontmatter, wikilinks). Pure file I/O —
  no MCP, no server, no database. Called by anansi-remember right before
  the anansi_ingest_atomized MCP call.
argument-hint: "[atomized content] [wiki dir, default: ~/llm-wiki]"
---

# wiki-ingest-atomized

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Client-side dual-write to the local Karpathy-style LLM wiki.

Takes the same `---`-delimited atomized block set that `anansi_ingest_atomized`
receives and writes each entity as a standalone markdown file in the local
`~/llm-wiki/` directory. Matches the server-side wiki format exactly so the
two are interchangeable.

Also maintains `index.md` (catalog) and `log.md` (append-only journal).

**Pure file I/O.** No MCP calls, no server, no database. The agent reads
from `~/llm-wiki/` directly — no Docker round-trip needed.

---

## When to invoke

- Called by `anansi-remember` after producing atomized content, right before
  the `anansi_ingest_atomized` MCP call
- Called standalone when you have atomized content and want to write it
  locally without a server

Do **not** invoke for:
- Raw source documents (use the full pipeline: para-process → sb-atomize → this)
- Reading from the wiki (use direct file reads or `wiki-recall`)

---

## Inputs

1. **Atomized content** — the full `---`-delimited block set (same as the
   `content` argument to `anansi_ingest_atomized`). Required.
2. **Wiki directory** — optional. Defaults to `~/llm-wiki` (expands to
   `$HOME/llm-wiki`). Pass an explicit path to override.

---

## Step 1 — Parse the atomized content

Split the content on `---` delimiters. Each segment is one entity block
(or the file header / Varys whispers block).

### File header

The first line of the content is:
```
<!-- anansi-atomize: <title> | <N> blocks | <date> -->
```

Extract the source title for the log entry.

### Entity blocks

Each entity block has the format:
```
### <address> <Name> [<entity_type>]

## Lede
<one-line summary>

## Why
<why this matters>

## Content
<detailed content>

## Edges
- <relation> <type>:<slug>
- <relation> <type>:<slug>
```

Parse each block to extract:
- `entity_type` — from the `[bracket]` tag (e.g. `organization`, `person`, `product`, `concept`)
- `name` — the display name after the address
- `slug` — kebab-case of the name (lowercase, non-alphanumerics → hyphens)
- `lede` — content under `## Lede`
- `why` — content under `## Why` (may be absent)
- `content` — content under `## Content` (may be absent)
- `edges` — list of relations from `## Edges`

### Varys whispers block

A block starting with `<!-- varys: whispers -->` is a cross-section signals
block. Skip it — it's not an entity.

---

## Step 2 — Write entity files

For each entity block, write a markdown file at:
```
{wiki_dir}/{slug}.{entity_type}.md
```

File format (matching the server-side wiki output exactly):

```markdown
---
entity_type: {entity_type}
name: {Name}
match_key: "{entity_type}:{slug}"
updated_at: "{ISO-8601 timestamp}"
---

# {Name}

{lede}

*{why}*

{content}

## Connections

- [[target-slug.target-type|Target Display Name]] `relation_type`
```

**Frontmatter rules:**
- `entity_type`, `name`, `match_key` — always present
- `updated_at` — current ISO-8601 timestamp
- No `anansi_id` — that gets populated when the entity is synced to the server

**Connections** — convert each edge line from the atomized block to a
wikilink with relation type:

| Atomized edge | Wiki Connections line |
|---|---|
| `- created concept:okf` | `- [[okf.concept\|OKF]] \`created\`` |
| `- related_to product:bigquery` | `- [[bigquery.product\|BigQuery]] \`related_to\`` |
| `- inspired_by concept:llm-wiki` | `- [[llm-wiki.concept\|LLM Wiki]] \`inspired_by\`` |

The display name is the title-case version of the slug. If the relation
type is the default (`related_to`), it can be omitted.

**Overwrite** existing files silently — the latest write wins.

---

## Step 3 — Update index.md

Read `{wiki_dir}/index.md` if it exists. The index is a catalog grouped
by entity_type, matching the server-side format:

```markdown
# Anansi LLM-Wiki — Index

## {entity_type}

- [[{slug}.{entity_type}|{Name}]] — {lede}
```

For each new entity, add an entry under its entity_type heading. Use the
lede as the one-line description. If the entity_type section doesn't exist,
create it. If the entry already exists (same slug), update the description.

Write the updated index back to `{wiki_dir}/index.md`.

---

## Step 4 — Append to log.md

Append one line to `{wiki_dir}/log.md`:

```
## [{date}] ingest | {source title} (+{N} notes)
```

Where `{date}` is today's date (YYYY-MM-DD), `{source title}` is from the
file header, and `{N}` is the number of entity blocks written.

Create `log.md` if it doesn't exist.

---

## Step 5 — Report

```
*wiki-ingest-atomized* — {source title}
• Wiki: {wiki_dir}
• Files written: {N}
• index.md: updated
• log.md: appended
```

## Done

- [ ] One `.md` file written per entity block in `{wiki_dir}/`
- [ ] Each file has correct frontmatter (`entity_type`, `name`, `match_key`, `updated_at`)
- [ ] Each file has `## Connections` section (may be empty)
- [ ] Wikilinks use correct format: `[[slug.type|Display Name]] \`relation\``
- [ ] `index.md` updated with new entries grouped by entity_type
- [ ] `log.md` appended with ingest entry

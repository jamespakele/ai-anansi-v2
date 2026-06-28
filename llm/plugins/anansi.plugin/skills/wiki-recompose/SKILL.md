---
name: wiki-recompose
description: >
  Reconstructs a source document from the local ~/llm-wiki/ files. Finds
  the outline record, walks its ## Connections to locate all entities that
  were created from that source, reads each entity's file, and assembles
  them back into a readable document. Pure local — no server trip needed.
argument-hint: "[source title or outline match_key]"
---

# wiki-recompose

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Reconstructs a source document from the local wiki files.

The server-side recompose reads the outline note from the database and
pulls each entity's content. The wiki version does the same from local
files — no MCP call, no Docker round-trip.

---

## When to invoke

- User asks "show me the recomposed document for that YouTube video"
- User says "recompose the OKF video notes"
- After ingesting a source, to see what got captured

---

## Inputs

- **source identifier** — one of:
  - Outline match_key: `outline:google-s-okf-the-simple-folder-replacing-vector-databases`
  - Source title: "Google's OKF: The Simple Folder Replacing Vector Databases"
  - Slug: `google-s-okf-the-simple-folder-replacing-vector-databases`

---

## Step 1 — Find the outline file

The outline file is at `{wiki_dir}/{slug}.outline.md`, where `{wiki_dir}`
defaults to `~/llm-wiki`.

If given a title, derive the slug (kebab-case). If given a match_key,
extract the slug from it.

If the outline file doesn't exist, stop and report.

---

## Step 2 — Read the outline

Read the outline file. Extract:
- `name` — the source title from frontmatter or `# {Name}`
- `connections` — all lines from the `## Connections` section

Each connection is a wikilink pointing to an entity that was created from
this source.

---

## Step 3 — Walk connections and read entity files

For each connection in `## Connections`:

1. Parse the wikilink: `[[{slug}.{type}|{Display Name}]]`
2. Read the entity file at `{wiki_dir}/{slug}.{type}.md`
3. Extract: `entity_type`, `name`, `lede`, `why`, `content`

Skip any entity whose file doesn't exist (it may have been evicted).

---

## Step 4 — Assemble the recomposed document

Build the output in this order:

```markdown
# {Source Title} (recomposed)

**wiki-recompose** · {N} entities · {date}

---

## Contents

1. {Entity 1 name} — {lede}
2. {Entity 2 name} — {lede}
...

---

## {Entity 1 name} ({entity_type})

{lede}

{why}

{content}

## Connections

{connections from the entity's file}

---

## {Entity 2 name} ({entity_type})
...
```

Group entities by type (organizations first, then people, then products,
then concepts) for readability.

---

## Step 5 — Report

```
*wiki-recompose* — {source title}
• Entities: {N}
• Source: local (~/llm-wiki/)
• Output: {inline or file path}
```

## Done

- [ ] Outline file found at `{wiki_dir}/{slug}.outline.md`
- [ ] All connected entity files exist and were read
- [ ] Recomposed document includes all entities grouped by type
- [ ] Each entity's lede, why, and content are included
- [ ] Connections from each entity file are preserved

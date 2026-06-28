---
name: wiki-relate
description: >
  Adds a connection (wikilink) between two entities in the local ~/llm-wiki/.
  Appends a line to the ## Connections section of the source entity's file,
  pointing to the target entity. Optionally syncs to the server afterward.
  Pure file I/O — no server call unless --sync is passed.
argument-hint: "[source match_key] [relation] [target match_key] [--sync]"
---

# wiki-relate

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Adds a typed connection between two entities in the local wiki.

The server-side `anansi_relate` creates edge records in the database.
The wiki equivalent appends a `[[wikilink]]` with a relation type to the
`## Connections` section of the source entity's file.

---

## When to invoke

- After creating two entities that should be linked
- To add a missing connection: "relate product:okf created_by organization:google"
- To update an existing connection's relation type

---

## Inputs

- **source** — match_key of the source entity (e.g. `product:okf`)
- **relation** — the edge type (e.g. `created_by`, `related_to`, `part_of`,
  `inspired_by`, `competes_with`)
- **target** — match_key of the target entity (e.g. `organization:google`)
- **`--sync`** — optional flag to also sync to the server after writing

---

## Step 1 — Resolve file paths

From each match_key (`type:slug`), derive the filename `{slug}.{type}.md`.
The wiki files are at `{wiki_dir}/{filename}`, where `{wiki_dir}` defaults
to `~/llm-wiki`.

If either file doesn't exist, stop and report which one is missing.

---

## Step 2 — Read the source file

Read the source entity's markdown file. Parse the frontmatter and body.

Locate the `## Connections` section. If it doesn't exist, append it at the
end of the file.

---

## Step 3 — Add the connection line

Append a line to the `## Connections` section:

```
- [[{target-slug}.{target-type}|{Target Name}]] `{relation}`
```

Where:
- `{target-slug}.{target-type}` — the target file reference
- `{Target Name}` — the display name (read from the target file's `# {Name}` heading)
- `` `{relation}` `` — the relation type in backticks

If the connection already exists (same target + same relation), skip it
and report "already connected".

---

## Step 4 — Optionally sync to server

If `--sync` was passed, call `wiki-sync` with `master=wiki` for the source
entity to push the updated connections to the server.

---

## Step 5 — Report

```
*wiki-relate* — {source name} → {target name}
• Relation: {relation}
• File: {wiki_dir}/{source filename}
• Server sync: {yes if --sync, else no}
```

## Done

- [ ] Source entity file exists
- [ ] Target entity file exists
- [ ] Connection line appended to `## Connections` section
- [ ] Wikilink format is correct: `[[{slug}.{type}|{Name}]] \`{relation}\``
- [ ] No duplicate connection created (same target + same relation)
- [ ] If `--sync` was passed, `wiki-sync` completed without error

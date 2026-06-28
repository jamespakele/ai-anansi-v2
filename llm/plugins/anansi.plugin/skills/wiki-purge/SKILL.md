---
name: wiki-purge
description: >
  Removes all local wiki files for a source and its entities. Finds the
  outline file, walks its ## Connections, deletes every connected entity
  file plus the outline itself, then updates index.md. Mirrors the
  server-side anansi_purge which deletes by source_id.
argument-hint: "[source slug or outline match_key] [--sync]"
---

# wiki-purge

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Removes all local wiki files for an imported source.

The server-side `anansi_purge` deletes all notes, edges, and contributions
by source_id. The wiki equivalent deletes the outline file and every entity
file that was created from that source.

---

## When to invoke

- After running `anansi_purge` on the server, to clean up the local wiki
- To remove a source and all its entities from the local wiki
- "purge the OKF video from the wiki"

---

## Inputs

- **source identifier** — slug or outline match_key
- **`--sync`** — optional flag to also call `anansi_purge` on the server

---

## Step 1 — Find the outline file

The outline file is at `{wiki_dir}/{slug}.outline.md`.

If it doesn't exist, stop and report.

---

## Step 2 — Walk connections and collect files

Read the outline's `## Connections` section. For each wikilink
`[[{slug}.{type}|...]]`, add `{slug}.{type}.md` to the delete list.

Add the outline file itself to the delete list.

---

## Step 3 — Delete the files

Delete each file in the delete list. Report how many were deleted and
how many didn't exist (already evicted).

---

## Step 4 — Update index.md

Remove the deleted entries from `index.md`.

---

## Step 5 — Optionally sync to server

If `--sync` was passed, call `anansi_purge` via MCP with the source_id
(if known) or the source title.

---

## Step 6 — Report

```
*wiki-purge* — {source title}
• Files deleted: {N}
• index.md: updated
• Server sync: {yes if --sync, else no}
```

## Done

- [ ] Outline file found and read
- [ ] All connected entity files deleted
- [ ] Outline file itself deleted
- [ ] `index.md` updated (entries removed)
- [ ] If `--sync` was passed, `anansi_purge` completed without error

---
name: wiki-rebuild
description: >
  Full rebuild of the local ~/llm-wiki/ from canonical Postgres state.
  Calls anansi_wiki_crawl to re-project every live note, regenerate
  index.md with the current format, and remove orphan files. Use after
  schema changes, index format changes, or to repair a corrupted wiki.
  ⚠ Resource-intensive — may take a while on large wikis.
argument-hint: "[--force]"
---

# wiki-rebuild

Full rebuild of the local Karpathy-style LLM wiki from canonical Postgres state.

Calls the `anansi_wiki_crawl` MCP tool to:
1. **Re-project every live note** — re-reads every note from Postgres and
   rewrites its wiki file with the current template format
2. **Rebuild `index.md`** — regenerates the catalog with the current PARA
   grouping (1. Projects, 2. Areas, 3. Resources, 4. Archives)
3. **Remove orphan files** — deletes wiki files whose notes no longer exist
4. **Append to `log.md`** — journals the crawl summary

---

## When to invoke

- After a schema change (new entity_type fields, new template format)
- After an index format change (like the PARA regrouping we just did)
- When the wiki appears stale, corrupted, or out of sync with the server
- On initial setup after enabling `[wiki] enabled = true`
- User says "rebuild the wiki", "re-sync the wiki", "fix the wiki"

Do **not** invoke for:
- Adding a single note (use `anansi_capture` — the wiki updates automatically)
- Reading from the wiki (use `wiki-recall`)

---

## ⚠ Resource warning

This operation re-projects **every live note** in the database. On a wiki
with thousands of notes, this means:

- **N database queries** (one per note, plus edges)
- **N file writes** (one per note file, plus index.md)
- **Duration**: proportional to note count. A 1400-note wiki may take
  30–60 seconds. Larger wikis scale linearly.

The crawl is **idempotent and safe** — it reads from Postgres (the source
of truth) and writes to the wiki (the projection). It will never lose data.
If interrupted, re-run it — it will pick up where it left off.

---

## Inputs

- **`--force`** — optional. Skip the confirmation prompt and proceed
  immediately. Use for scripted or automated rebuilds.

---

## Step 1 — Warn and confirm

Present this warning to the user:

```
⚠ wiki-rebuild will re-project ALL live notes from Postgres.
  This may take a while and is resource-intensive.

  What will happen:
  • Every note file will be re-read from the database and rewritten
  • index.md will be regenerated with the current PARA format
  • Orphan files (notes that no longer exist) will be removed
  • log.md will be appended with a crawl summary

  This is safe and idempotent. Postgres is the source of truth.
```

If `--force` was not passed, ask the user to confirm before proceeding.

---

## Step 2 — Call anansi_wiki_crawl

Call the `anansi_wiki_crawl` MCP tool with no arguments.

The tool returns a JSON response with:
- `notes_projected` — number of note files written
- `evicted` — number of notes evicted (under tipping-point caps)
- `orphans_removed` — number of orphan files deleted
- `errors` — number of errors encountered

---

## Step 3 — Report

```
*wiki-rebuild* — complete
• Notes projected: {N}
• Orphans removed: {N}
• Errors: {N}
• Duration: {approx time}
• index.md: regenerated with PARA grouping
• log.md: appended
```

---
name: wiki-sync
description: >
  Syncs a local wiki entity to or from the Anansi server. Reads the
  entity's markdown file from ~/llm-wiki/, wraps it as an atomized block,
  and calls anansi_ingest_atomized (master=wiki) or pulls from the server
  and writes locally (master=anansi). Default: master=wiki (local first).
argument-hint: "[match_key] [master=wiki|anansi]"
---

# wiki-sync

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Syncs a single entity between the local `~/llm-wiki/` and the Anansi server.

Two directions:
- **master=wiki** (default): read local file → build atomized block →
  `anansi_ingest_atomized` → server gets the local version
- **master=anansi**: `anansi_get` from server → write local file →
  local wiki gets the server version

---

## When to invoke

- After `wiki-ingest-atomized` writes entities locally, to push them to
  the server (`master=wiki`)
- After a local edit to a wiki file, to sync the change to the server
  (`master=wiki`)
- To pull a server-side entity into the local wiki for offline reading
  (`master=anansi`)
- On demand: "sync product:okf to the server"

---

## Inputs

- **match_key** — e.g. `product:okf`, `person:andrej-karpathy`
- **master** — `wiki` (default) or `anansi`

### Master resolution order

1. If the user explicitly says "sync to the server" or "push to anansi" →
   `master=wiki`.
2. If the user says "pull from the server" or "download from anansi" →
   `master=anansi`.
3. If the context makes it obvious (e.g. you just edited the local file
   and now want to sync) → infer from context.
4. If it's ambiguous → default to `master=wiki`. The server has
   versioning and backups, so a bad sync can be rolled back there.
   Local edits have no such safety net — default to pushing local
   changes up rather than overwriting them.

---

## Step 1 — Resolve the file path

Derive the filename as `{slug}.{type}.md` from the match_key.
The wiki file is at `{wiki_dir}/{filename}`, where `{wiki_dir}` defaults to
`~/llm-wiki` (expand `~` to `$HOME`).

---

## Step 2 — Sync by direction

### master=wiki (local → server)

1. Read the local wiki file at `{wiki_dir}/{filename}`.
2. Parse the frontmatter (`entity_type`, `name`, `match_key`) and body
   (`# {Name}`, lede, `*{why}*`, content, `## Connections`).
3. Build an atomized block:

   ```markdown
   ### 1.1 {Name} [{entity_type}]

   ## Lede
   {lede}

   ## Why
   {why}

   ## Content
   {content}

   ## Edges
   {connections, converted from [[slug.type|Name]] `rel` to rel type:slug format}
   ```

4. Wrap in the atomized file header and call `anansi_ingest_atomized`:

   ```
   content      = {built atomized content}
   source       = "skill"
   ```

### master=anansi (server → local)

1. Call `anansi_get` with the match_key to fetch the entity from the server.
2. Extract `entity_type`, `name`, `lede`, `why`, `content`, and any edges
   from the response.
3. Write the local wiki file in the standard format (same as
   `wiki-ingest-atomized` Step 2).
4. Update `index.md` and `log.md`.

---

## Step 3 — Report

```
*wiki-sync* — {Name}
• Match key: {match_key}
• Direction: {master} → {the other}
• File: {wiki_dir}/{filename}
• Status: synced
```

## Done

- [ ] Source file exists at `{wiki_dir}/{filename}`
- [ ] Atomized block constructed with correct format
- [ ] `anansi_ingest_atomized` called with `source: "skill"`
- [ ] Response indicates success (no error returned)

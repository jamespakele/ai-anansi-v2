---
name: anansi-purge
description: >
  Deletes all notes, edges, and contributions from a specific Anansi import by
  source_id, then removes the source record itself. Irreversible — use
  anansi_search or Datasette to find the source_id before running. Invoke when
  the user says "purge this source", "delete this import", "remove source_id
  [uuid]", "/anansi-purge", "purge from anansi", "delete all notes from this
  import", or "clean up this source from the vault".
argument-hint: "[source_id UUID]"
---

# anansi-purge

Purge all notes, edges, and contributions tied to a single import from the
Anansi knowledge base. The source record is removed too. This is irreversible.

---

## Step 1 — Identify the source

The user must provide a source identifier. Accept either:
- **source_id** (UUID) — for the server-side purge
- **source slug or title** — for the local wiki purge (e.g. "google-s-okf..."
  or "Google's OKF: The Simple Folder Replacing Vector Databases")

If neither is provided, stop and ask:

> "What source do you want to purge? Give me the source_id (UUID) or the
> source title/slug."

---

## Step 2 — Wiki check (runs first if wiki is enabled)

If the local wiki exists at `~/llm-wiki/` and a source slug or title was
provided, delegate to `wiki-purge` first to clean up local files:

1. Read `../wiki-purge/SKILL.md`.
2. Execute it with the source slug or title.
3. Then proceed to Step 3 to also purge from the server.

If only a source_id was provided, skip the wiki step — can't resolve to
local files without a DB query.

---

## Step 3 — Confirm before firing

Before calling `anansi_purge`, show the user what's about to happen and ask
for confirmation:

```
About to purge source_id: {source_id}
This will permanently delete all notes, edges, and contributions from this
import. This cannot be undone.

Confirm? (yes / no)
```

Only proceed if the user confirms.

---

## Step 4 — Call `anansi_purge`

Always include `"source": "skill"` in the call payload.

Call `anansi_purge` with:

```json
{
  "source_id": "<source_id from user>",
  "source": "skill"
}
```

---

## Step 5 — Report

```
*Anansi* — purge complete
* source_id: {source_id}
* Local wiki: {cleaned if wiki was enabled}
* All notes, edges, and contributions removed.
```

If the call returns an error, show it clearly and do not retry without the
user's instruction.

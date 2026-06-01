---
name: anansi-purge
description: >
  Deletes all notes, edges, and contributions from a specific Anansi import by
  source_id, then removes the source record itself. Irreversible - use
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

## Step 1 — Confirm source_id

The user must provide a source_id (UUID). If not provided, stop and ask:

> "What's the source_id to purge? You can find it with `anansi_search` or in
> Datasette."

Do not guess or infer a source_id. It must be explicit.

---

## Step 2 — Confirm before firing

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

## Step 3 — Call `anansi_purge`

Always include `"source": "skill"` in the call payload.

Call `anansi_purge` with:

```json
{
  "source_id": "<source_id from user>",
  "source": "skill"
}
```

---

## Step 4 — Report

```
*Anansi* — purge complete
* source_id: {source_id}
* All notes, edges, and contributions removed.
```

If the call returns an error, show it clearly and do not retry without the
user's instruction.

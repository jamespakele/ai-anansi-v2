---
name: wiki-purge
description: >
  ⚠ DEPRECATED. Use anansi_purge (MCP tool) to delete notes by source_id
  from the database. The server's WikiStore::remove_note + refresh_index
  will clean up the wiki files automatically. This skill is kept for
  reference only and will be removed in a future cleanup.
argument-hint: ""
---

# wiki-purge — ⚠ DEPRECATED

**This skill is deprecated.** The correct flow is:

1. Call `anansi_purge(source_id)` (MCP tool) to delete all notes, edges,
   and contributions by source_id from the database
2. The server's `WikiStore::remove_note` deletes each wiki file
3. The server's `refresh_index` removes the entries from `index.md`

Deleting local wiki files directly and then optionally syncing to the
server is backwards — the server is the source of truth. If you delete
local files without purging the database, the next crawl will recreate them.

## What to use instead

| Instead of this skill | Use this |
|---|---|
| Deleting a source and all its entities | `anansi_purge(source_id)` — deletes from Postgres, wiki follows |
| Cleaning up after a server-side purge | Not needed — the server handles wiki cleanup automatically |
| Removing a single note | `anansi_delete_note(match_key)` or `anansi_archive_note(match_key)` |

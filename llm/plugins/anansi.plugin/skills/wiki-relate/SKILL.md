---
name: wiki-relate
description: >
  ⚠ DEPRECATED. Use anansi_relate (MCP tool) to add edges in the database.
  The server's WikiStore::materialize will update the wiki file's
  ## Connections section automatically on the next capture or crawl.
  This skill is kept for reference only and will be removed in a future
  cleanup.
argument-hint: ""
---

# wiki-relate — ⚠ DEPRECATED

**This skill is deprecated.** The correct flow is:

1. Call `anansi_relate` (MCP tool) to add the edge in the database
2. The server's `WikiStore::materialize` updates the wiki file's
   `## Connections` section automatically on the next capture or crawl

Editing the local wiki file directly and then syncing to the server is
brittle — the server is the source of truth, and the wiki is a projection.
Edits to the local file that aren't reflected in the database will be
overwritten on the next materialize.

## What to use instead

| Instead of this skill | Use this |
|---|---|
| Adding a connection between two entities | `anansi_relate(source, relation, target)` — adds the edge in Postgres |
| Updating an existing connection | `anansi_relate` with the updated relation type |
| Removing a connection | `anansi_unrelate` (if available) or `anansi_capture` with the updated edges list |

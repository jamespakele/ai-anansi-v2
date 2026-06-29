---
name: wiki-ingest-atomized
description: >
  ⚠ DEPRECATED. The server-side WikiStore::materialize now handles wiki
  file writes automatically on every anansi_capture. Use anansi_capture
  directly — the wiki stays in sync. This skill is kept for reference
  only and will be removed in a future cleanup.
argument-hint: ""
---

# wiki-ingest-atomized — ⚠ DEPRECATED

**This skill is deprecated.** The Rust server now handles wiki materialization
automatically via `WikiStore::materialize` on every `anansi_capture` call.

## What to use instead

| Instead of this skill | Use this |
|---|---|
| Writing atomized content to the wiki | `anansi_capture` — the server writes the wiki file automatically |
| Writing to the wiki without a server | Not needed — the wiki is a projection of the server. Run `wiki-rebuild` to rebuild from Postgres |

## Why it's deprecated

Before the Rust server had the wiki dual-write built in (Build-12), the LLM
had to write wiki files client-side and then separately sync them to the
server. Now the server does both in one step: `anansi_capture` writes to
Postgres AND materializes the wiki file. The client-side dual-write is
redundant and can produce drift if the two paths disagree.

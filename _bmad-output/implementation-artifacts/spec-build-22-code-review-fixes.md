---
title: 'Anansi v2 — Build 22: code-review fixes (wiki↔Postgres drift)'
type: 'bugfix'
created: '2026-06-24'
status: 'done'
baseline_commit: 'b7a4b2d'
context:
  - _bmad-output/implementation-artifacts/deferred-work.md
---

## Intent

**Problem:** A high-effort multi-agent code review of the whole `feat/llm-wiki` branch (Builds 12–21) found a cluster of wiki↔Postgres drift bugs that the per-build reviews couldn't see because they're cross-build integration gaps.

**Approach:** Fix the default-config correctness bugs (and two easy wins); defer the capped-mode-only, perf, and quality findings to `deferred-work.md`.

## Fixes (this build)

- **`tool_update_note` never re-projected to the wiki** (`mcp.rs`) — now removes the old file on a name/type change and re-materializes the note (non-fatal). Editing/renaming a note no longer leaves stale or orphaned wiki files.
- **`tool_relate` never re-materialized its endpoints** (`mcp.rs`) — now materializes both connected notes so the new typed wikilink appears in each note's `## Connections` immediately (non-fatal, only on actual insert).
- **Single per-note error froze ALL crawl reconciliation** (`wiki.rs`) — the uncapped (default) crawl now derives its GC keep-set AND index authoritatively from `all_note_summaries`, independent of per-note projection success, so one bad row can't permanently freeze index/GC. The capped path keeps its conservative `project_failed` skip.
- **Caps set + crawl disabled → `index.md` never updated** (`wiki.rs`) — added `WikiStore::crawl_enabled` + `crawl_owns_index()`; capture/refresh now defer the index to the crawl ONLY when the crawl actually runs. Caps-without-crawl keeps maintaining `index.md` synchronously (no crawl ⇒ no eviction ⇒ the all-live index is correct).
- **`export_wiki` shipped a bogus `## [date] crawl` line in `log.md`** (`export.rs`) — the export now drops `log.md` from the bundle (it's a content snapshot; the local install keeps its own journal).
- **Lint analyzed archived neighbors the wiki excludes** (`lint.rs`) — neighbor resolution now skips `archive-*` notes, matching the projection.

## Deferred (see deferred-work.md)

Capped-mode dangling wikilinks; eviction budget double-count on same-name `:variant` notes; warmth-ordering timestamp-format mismatch; capture inline full-table-scan perf; and quality cleanups (dead `live_note_ids_by_access`, edge_other/cutoff/JSON dedup, MCP envelope helper).

## Verification

- `cargo check` — clean.
- `cargo test --lib` — 65 pass; only the 3 pre-existing path-dependent `prompt::tests` fail (unrelated, already deferred).
- `cargo test --no-run` — full crate compiles.

# Anansi Ingest Bug Report
**Date:** April 24, 2026  
**Reporter:** Claude (Cowork session, james@pakele.ai)  
**Vault:** `/vault/` (anansi v2 daemon)  
**Source document:** `POW_Board_Briefing_April2026.md`

---

## Summary

`anansi_ingest` times out on large documents (~25KB, 74-leaf TOC), partially commits entity notes to the database, then fails with a `UNIQUE constraint` error on all subsequent re-ingest attempts. There is no recovery path available through the MCP interface.

---

## What Happened — Step by Step

### Attempt 1 — content + filename (timed out)

Called `anansi_ingest` with full augmented document content (~24,787 chars, 74 TOC leaves) and `filename: "POW_Board_Briefing_April2026.md"`.

**Result:** MCP tool timed out after 60 seconds.

**Side effect:** The daemon had already begun processing and committed at least 3 entity notes to the database before the timeout killed the connection:

| match_key | entity_type | name | id |
|---|---|---|---|
| `note:key-people` | note | Key People | `82df012e-1675-4fd3-be1f-075040fec8be` |
| `person:ron-nishihara` | person | Ron Nishihara | `6337e192-0136-4973-a70e-5ff7a95519ad` |
| `organization:dynamic-community-solutions` | organization | Dynamic Community Solutions | `1e036adb-1749-4796-b5f8-6a4d6c62d3f3` |

These notes were written and are queryable via `anansi_search` and `anansi_get`. **No source document record was created. No outline was created. No edges were written (`edge_count: 0` on Ron Nishihara).**

---

### Attempt 2 — source_path (parse error)

Called `anansi_ingest` with `source_path: "/sessions/great-loving-ramanujan/mnt/DCS Board Meetings/POW_Board_Briefing_April2026.md"`.

**Result:** `Ingest failed: parsing source file: /sessions/great-loving-ramanujan/mnt/DCS Board Meetings/POW_Board_Briefing_April2026.md`

**Diagnosis:** The daemon does not have filesystem access to the Cowork session sandbox path. The `/sessions/` path is not mounted inside the daemon's container/environment.

---

### Attempt 3 — source_path without spaces (parse error)

Copied file to `/sessions/great-loving-ramanujan/POW_Board_Briefing_April2026.md` and retried with that path.

**Result:** Same parse error. Confirms the daemon cannot access any path outside `/vault/`.

---

### Attempt 4 — content + filename, cleaned content (UNIQUE constraint error)

Found and stripped a trailing `\xef\xbf\xbd` (UTF-8 replacement character U+FFFD) from the end of the document — the original file ended with a broken character. Retried with clean content and a slightly simplified body (special Unicode characters like `ʻ` okinas normalized in the body text, preserved in the frontmatter TOC).

**Result:** `Ingest failed: error returned from database: (code: 2067) UNIQUE constraint failed: notes.match_key`

**Diagnosis:** The daemon attempts to `INSERT` entity notes from the new TOC, but `person:ron-nishihara`, `organization:dynamic-community-solutions`, and `note:key-people` already exist from Attempt 1. The constraint fires before any new notes are written or the source document is recorded.

---

## Root Cause

Two separate bugs:

### Bug 1 — No timeout / chunking on large ingest (severity: medium)

The MCP tool call has a hard 60-second timeout. For a 74-leaf TOC with full document content, the daemon exceeds this threshold during Pass 3/4 processing. The daemon should either:
- Process asynchronously and return a job ID immediately, OR
- Accept the TOC-only pass (with pre-built TOC, skip Pass 1) fast enough to beat the timeout

**The pre-built TOC should make this faster** — but apparently it still triggers expensive downstream passes that exceed 60 seconds.

### Bug 2 — Non-idempotent entity note writes (severity: high)

Entity notes are written using plain `INSERT` into the `notes` table. When the same entity appears in a second ingest (or a re-ingest after a partial failure), the insert fails with a UNIQUE constraint on `match_key`.

**Expected behavior:** Entity notes should use `INSERT OR REPLACE` (SQLite upsert) or `ON CONFLICT(match_key) DO UPDATE SET ...`. This makes ingest idempotent — re-ingesting a document updates existing entity notes rather than failing.

**The partial-commit side effect of Bug 1 makes Bug 2 fatal:** a timed-out ingest leaves orphaned entity notes that block all future ingestion of any document containing those entities.

---

## Observed Vault State After Failed Ingests

```
anansi_search("Ron Nishihara")
→ person:ron-nishihara  ✅ exists, summaries populated, edge_count: 0, source_count: 1

anansi_search("Dynamic Community Solutions")
→ organization:dynamic-community-solutions  ✅ exists

anansi_search("Key People")
→ note:key-people  ✅ exists

anansi_search("Continest")          → ❌ not found
anansi_search("G70 Engineering")    → ❌ not found
anansi_search("Kawika")             → ❌ not found
anansi_search("DLNR HECO Holoholo") → ❌ not found
anansi_search("Fast Case Scenario") → ❌ not found
anansi_search("POW board briefing source") → ❌ not found
```

**Summary:** 3 entity notes committed, 71 entity notes missing, 0 source documents, 0 outline, 0 edges.

---

## Source Document State

The source file **has been fully augmented** with a valid `anansi_toc` frontmatter block (74 leaves, all typed, all hinted). The YAML was validated with Python's `yaml.safe_load` — it parses correctly.

File location (Cowork workspace): `/sessions/great-loving-ramanujan/mnt/DCS Board Meetings/POW_Board_Briefing_April2026.md`

The file is ready for ingest the moment the constraint issue is resolved.

---

## Recommended Fixes

### Fix 1 — Upsert entity notes (required)

In the ingest pipeline, wherever entity leaf notes are written to the `notes` table, change:

```sql
-- Current (breaks on re-ingest)
INSERT INTO notes (match_key, name, entity_type, ...) VALUES (?, ?, ?, ...)

-- Fix: SQLite upsert
INSERT INTO notes (match_key, name, entity_type, ...)
VALUES (?, ?, ?, ...)
ON CONFLICT(match_key) DO UPDATE SET
  name = excluded.name,
  summary_1 = excluded.summary_1,
  summary_5 = excluded.summary_5,
  updated_at = CURRENT_TIMESTAMP;
```

This makes ingest idempotent and safe to retry after partial failures.

### Fix 2 — Clear partial state on timeout / rollback (required)

The ingest operation should run inside a database transaction. If the MCP connection drops or the call times out, the transaction should roll back — preventing orphaned partial commits. 

```python
# Pseudocode
with db.transaction():
    write_source_record(source_id, ...)
    for leaf in toc_leaves:
        upsert_entity_note(leaf)
    write_outline(source_id, ...)
    write_edges(...)
# commit only on full success; rollback on any failure
```

### Fix 3 — Return job ID for async processing (recommended for large docs)

For documents with large TOCs, ingest should return immediately with a `job_id` and process asynchronously:

```json
{ "status": "queued", "job_id": "abc123", "source_id": "..." }
```

The MCP caller can poll `anansi_job_status(job_id)` to check completion. This eliminates the 60-second timeout risk entirely.

---

## Immediate Recovery Steps (Manual)

To unblock the current document without daemon changes:

```sql
-- Connect to the vault SQLite database (web.db or equivalent)
DELETE FROM notes WHERE match_key IN (
  'note:key-people',
  'person:ron-nishihara',
  'organization:dynamic-community-solutions'
);
```

Then re-run `anansi_ingest` with `content + filename`. The full augmented document (with pre-built 74-leaf TOC) is at:

```
/sessions/great-loving-ramanujan/mnt/DCS Board Meetings/POW_Board_Briefing_April2026.md
```

---

## MCP Tool Signatures (for reference)

```json
{
  "name": "anansi_ingest",
  "parameters": {
    "content": "string (raw markdown)",
    "filename": "string",
    "source_path": "string (absolute path — only works if path is inside /vault/)"
  }
}
```

Note: `source_path` only works for paths the daemon can reach. The Cowork session sandbox (`/sessions/...`) is not accessible to the daemon. Use `content + filename` for all Cowork-sourced documents.

---

*Report generated by Claude Cowork · james@pakele.ai · April 24, 2026*

---
name: web-tender
description: >
  Runs the Anansi Web Tender — a background maintenance crawler that scans the
  knowledge graph for issues and performs high-confidence auto-fixes directly,
  flagging only lower-confidence items to the queue for review.
  Supports dry-run mode (report only) and apply mode (execute fixes).
  Triggers: "web-tender", "/web-tender", "run the web tender", "tend the web",
  "run maintenance", "crawl for issues".
argument-hint: "[--dry-run | --apply]"
---

# web-tender

Runs the Anansi Web Tender — a background maintenance crawler that scans the
knowledge graph for structural issues, deduplication candidates, broken edges,
orphan nodes, stale entries, circular references, template drift, and edge
inference opportunities.

---

## Overview

The Web Tender is a periodic maintenance pass over the entire knowledge graph.
It performs the following scans in order:

| # | Scan               | What it checks                                      | Auto-fix? |
|---|--------------------|------------------------------------------------------|-----------|
| 1 | Edge Integrity     | Dangling edges whose source or target no longer exist | Yes       |
| 2 | Duplicate Edges    | Exact duplicate (source, target, type) rows          | Yes       |
| 3 | Wikilink Validity  | Broken `[[wikilinks]]` in note content               | Yes       |
| 4 | Exact Dedup        | Notes with same normalized_name + entity_type        | Yes       |
| 5 | Conflicting Edges  | Contradictory edge pairs (e.g. A reports_to B and B reports_to A) | Queue |
| 6 | Orphan Nodes       | Entities with zero edges and no incoming references  | Queue     |
| 7 | Stale Entries      | Notes with empty why and content                     | Queue     |
| 8 | Circular References | Edge cycles (A→B→C→A)                               | Queue     |
| 9 | Type Consistency   | Edge types that don't match endpoint entity types    | Queue     |

High-confidence auto-fixes (1–4) are applied directly in apply mode.
Lower-confidence items (5–9) are written to the `tender_queue` table with a
category, severity, confidence score, and status for human review.

---

## Usage

### `--dry-run` (default)

Reports all findings without modifying the database:

```
/web-tender --dry-run
```

The tender runs all 9 scans and prints a summary to the chat. No entities,
edges, or fields are modified.

### `--apply`

Executes high-confidence auto-fixes directly and writes lower-confidence
findings to the queue:

```
/web-tender --apply
```

**Auto-fixes applied directly:**
- **Dangling edges** — removed automatically
- **Duplicate edges** — extras removed, one kept
- **Broken wikilinks** — `[[missing]]` stripped to plain text
- **Exact deduplication matches** — merged automatically (see `deduplication.md`)

**Queued for human review:**
- Conflicting edges
- Orphan nodes
- Stale/empty notes
- Circular references
- Type consistency violations

---

## Status Check

To check whether a tender run is currently in progress or when the last run
completed, query the `tender_queue` table:

```sql
SELECT
  MAX(created_at) AS last_run,
  COUNT(*) FILTER (WHERE status = 'open') AS open_items,
  COUNT(*) FILTER (WHERE status = 'resolved') AS resolved_items,
  COUNT(*) FILTER (WHERE status = 'dismissed') AS dismissed_items
FROM tender_queue;
```

A summary of the last run is printed at the end of every `--dry-run` or
`--apply` invocation.

---

## Log Review

Each tender run produces a run log. To review past runs:

```sql
SELECT
  DATE_TRUNC('hour', created_at) AS run_hour,
  category,
  COUNT(*) AS items_found,
  COUNT(*) FILTER (WHERE status = 'resolved') AS auto_resolved
FROM tender_queue
GROUP BY run_hour, category
ORDER BY run_hour DESC, category;
```

The chat output also includes a per-category breakdown at the end of every run.

---

## Hard Rules

1. **Idempotent** — Running `--dry-run` twice produces the same results (unless
   the underlying data changed). Running `--apply` twice on the same data is
   safe: already-resolved items are skipped.
2. **Batch mode** — All scans run to completion in a single pass. The tender
   does not stop mid-run.
3. **Read-only by default** — `--dry-run` never writes to any table.
4. **Auto-fixes are applied directly** — high-confidence fixes (dangling edges,
   duplicate edges, broken wikilinks, exact dedup) execute without queueing.
   Only lower-confidence items go to the queue.
5. **Hard deletes** — Merged duplicate notes are hard-deleted immediately.
   There is no soft-delete or 30-day window.
6. **Rate-limited** — At most 10 dedup merges per `--apply` run. Remaining
   candidates wait for the next run.
7. **Log everything** — Every finding, auto-fix, and skipped item is recorded
   in `tender_queue` with a timestamp and confidence score.
8. **Fail open** — If a scan errors, log the error to the run summary and
   continue to the next scan. Do not abort the entire run.

# Web Tender — Handoff Document

## Overview

Build three components that together form the **Anansi Web Tender** — a background maintenance crawler that keeps the knowledge graph healthy, plus an audit/reporting layer that surfaces anything the crawler can't auto-resolve.

The name comes from **Anansi the spider** — the web tender is the spider that tends its web, repairing broken strands, removing debris, and keeping the whole structure strong.

---

## Components

### 1. `web-tender` — The Crawler

A background process (cron job inside the container or systemd timer on the VPS) that periodically crawls the Anansi database and performs maintenance.

#### Checks & Actions

| Check | What it does | Auto-fix? | Confidence |
|-------|-------------|------------|------------|
| **Template alignment** | Compares each note's fields against the current template for its `entity_type`. If fields are missing, renamed, or in the wrong format, re-atomizes the note through the current `sb-atomize` pipeline. | Yes | High |
| **Edge integrity** | For every edge in the `edges` table, verifies both `source_id` and `target_id` still exist in `notes`. Removes dangling edges. | Yes | High |
| **Duplicate edges** | Detects identical `(source_id, target_id, edge_type)` rows and removes duplicates, keeping one. | Yes | High |
| **Conflicting edges** | Detects contradictory edges (e.g., `works_at` and `competitor_of` between the same pair). Flags for review. | No | Low |
| **Edge inference** | Scans note `content` for mentions of known entities (by name or match_key) and suggests missing edges. | No (flags) | Medium |
| **Index / outline sync** | Verifies that `index.md` and per-type outline notes list every entity that exists. Adds missing entries, removes entries for deleted notes. | Yes | High |
| **Wikilink validity** | Scans all note content for `[[wikilinks]]` and verifies the target exists. Removes or flags broken links. | Yes (remove) | High |
| **Normalization rules** | Reads rules from `rules/normalization.md` and applies them (name casing, punctuation, slug format, etc.). Rules can change between runs; the tender adapts. | Yes | High |
| **Deduplication (exact)** | Finds notes with identical `(normalized_name, entity_type)` and merges them (union `why`/`content`, combine edges, keep oldest, tombstone the duplicate). | Yes | High |
| **Deduplication (semantic)** | Uses vector similarity (embedding cosine distance) to find near-duplicates. Flags pairs above a configurable threshold for review. | No (flags) | Medium |
| **Orphan detection** | Finds notes with zero incoming and zero outgoing edges. Flags for review. | No (flags) | Low |
| **Stale / empty notes** | Finds notes where `why` and `content` are both empty or trivially short. Flags for review. | No (flags) | Low |
| **Source integrity** | Verifies every note's `source_id` still exists in the `sources` table. Flags orphans. | No (flags) | Medium |
| **Circular references** | Detects edge cycles (A → B → C → A) using BFS. Flags for review. | No (flags) | Low |
| **Type consistency** | Validates that edge types are semantically valid for the entity types they connect (e.g., `reports_to` should connect `person→person`, not `org→org`). | No (flags) | Medium |

#### Processing Model

- **Batch mode**: Processes N notes per run (configurable, default 100), ordered by `updated_at` ascending (oldest first).
- **Idempotent**: Running twice on the same data produces the same result.
- **Dry-run mode**: `--dry-run` reports everything it *would* do without modifying the DB.
- **Apply mode**: `--apply` executes the auto-fixes and writes to the queue.

#### Output

1. **Auto-fixes**: Applied directly to the DB.
2. **Flags**: Written to the `tender_queue` table (see below) with `status = 'open'`.
3. **Log**: Appends a summary to `tender-log.md` (or a `tender_log` table) with counts of what was fixed, flagged, skipped, and errored.

---

### 2. `tender_queue` — The Queue Table

A database table that stores items flagged by the web tender for human review.

```sql
CREATE TABLE tender_queue (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  category      TEXT NOT NULL,        -- 'dedup', 'broken_edge', 'orphan', 'stale', 'circular', 'template_drift', 'edge_inference', 'type_consistency'
  severity      TEXT NOT NULL DEFAULT 'info',  -- 'low', 'medium', 'high', 'info'
  match_key     TEXT,                 -- the primary entity involved
  related_keys  TEXT[],               -- other entities involved (e.g., the duplicate pair, the missing target)
  description   TEXT,                 -- human-readable summary of the issue
  confidence    REAL,                -- 0.0 to 1.0
  status        TEXT NOT NULL DEFAULT 'open',  -- 'open', 'resolved', 'dismissed'
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_at   TIMESTAMPTZ,
  resolved_by   TEXT                  -- 'auto', 'manual', or NULL
);

CREATE INDEX idx_tender_queue_status ON tender_queue(status);
CREATE INDEX idx_tender_queue_category ON tender_queue(category);
CREATE INDEX idx_tender_queue_severity ON tender_queue(severity);
```

**Lifecycle:**
- Web tender inserts rows with `status = 'open'`.
- If the tender encounters the same issue on a subsequent run and it's still open, it updates `updated_at` (so you know it's still being flagged).
- Audit skill reads `WHERE status = 'open'`.
- User or audit skill can set `status = 'resolved'` or `'dismissed'`.

---

### 3. `web-tender-audit` — The Report Skill

An on-demand skill that reads the `tender_queue` table and produces a human-readable markdown report of everything still needing attention.

#### Report Format

```markdown
# Web Tender Audit — {date}

## Summary
- Open items: {N}
- Auto-fixed this run: {N}
- Errors: {N}

## 🟡 Low confidence dedup candidates ({N})
| # | Match keys | Similarity | Lede overlap | Action |
|---|---|---|---|---|
| 1 | `person:john-doe`, `person:johnathan-doe` | 92% | 85% | Merge? |

## 🔴 Broken edges ({N})
| # | Source | Target | Edge type | Issue |
|---|---|---|---|---|
| 1 | `person:james-pakele` → `org:iq360` | not found | `works_at` | Target deleted |

## 🟠 Orphan notes ({N})
| # | Match key | Lede | Created |
|---|---|---|---|
| 1 | `note:draft-thoughts` | "Some random thoughts..." | 2026-03-15 |

## ⚪ Stale/empty notes ({N})
| # | Match key | Content length |
|---|---|---|
| 1 | `person:unknown-contact` | 0 bytes |

## 🔵 Circular reference candidates ({N})
None found.

## 🟣 Template drift ({N})
All notes match current templates.
```

#### Behavior

- **Dry-run by default**: Reads the queue, queries the DB to verify each item is still relevant (the user may have fixed it manually), and renders the report. Does not modify the queue.
- **`--resolve` flag**: Marks items as `resolved` after the report is generated (for items the user confirms are done).
- **`--dismiss` flag**: Marks items as `dismissed` (for items the user decides are not worth fixing).
- **Groups by category** and sorts by severity (high first), then by age (oldest first).

---

## Rules Files

Normalization and deduplication rules live in markdown files under `rules/` so they can be updated without code changes:

```
rules/
├── normalization.md        # Name casing, punctuation, slug rules
├── deduplication.md         # Thresholds for exact vs. semantic match
├── edge-inference.md       # When to auto-add edges from content
└── template-mapping.md     # Which template fields map to which DB columns
```

The web tender reads these at the start of each run. If a rule file is updated, the next crawl automatically applies the new rules.

---

## Implementation Order

1. **`tender_queue` table** — migration to create the table and indexes.
2. **Rules files** — create the `rules/` directory with initial defaults.
3. **`web-tender` skill** — the crawler, starting with the highest-confidence checks (template alignment, edge integrity, duplicate edges, index sync, wikilink validity, normalization, exact dedup) and adding lower-confidence checks later.
4. **`web-tender-audit` skill** — the report generator that reads the queue and produces the markdown report.
5. **Cron / timer** — schedule the web tender to run nightly (or hourly) on the VPS.

---

## Key Constraints

- **DB is the source of truth** — the wiki is regenerated from the DB, not the other way around.
- **Never destructive without a queue entry** — every auto-fix should be logged. Anything the tender can't auto-fix with high confidence goes into the queue.
- **Idempotent** — running the tender twice should be safe.
- **Rules-driven** — behavior changes by editing markdown files, not code.
- **Batch-processed** — don't try to crawl the entire DB in one run; process in batches to avoid overwhelming the server.

---

## Related Files

| File | Purpose |
|------|---------|
| `deploy/syncthing-bootstrap.py` | VPS Syncthing setup (already deployed) |
| `llm/plugins/anansi.plugin/skills/sync-setup/SKILL.md` | Client-side sync setup skill (already deployed) |
| `llm/plugins/anansi.plugin/skills/web-tender/SKILL.md` | **To be created** — the web tender skill |
| `llm/plugins/anansi.plugin/skills/web-tender-audit/SKILL.md` | **To be created** — the audit skill |
| `rules/normalization.md` | **To be created** — normalization rules |
| `rules/deduplication.md` | **To be created** — dedup thresholds |
| `rules/edge-inference.md` | **To be created** — edge inference rules |
| `rules/template-mapping.md` | **To be created** — template-to-DB mapping |
| `server/migrations/XXXXXX_create_tender_queue.up.sql` | **To be created** — migration for the queue table |

---
id: merge-strategy
type: rule
status: active
version: "2.0"
last_updated: 2026-04-23
---

# Merge Strategy Rules

Every entity type belongs to exactly one of three merge categories. The
category determines what happens when a second source mentions the same entity.

## Category 1: Pure Atomic

**Types:** person, concept, topic, area, note

**Rule:** First writer fills the identity fields. Later writers fill blanks
only. Conflicts (two sources give different values for the same field) are
logged to `source_contributions` with `contribution_type = 'conflict'` —
the existing value is NOT overwritten.

**Algorithm:**
```
for each identity field:
  if existing[field] is blank or "[not mentioned]":
    existing[field] = new[field]          # fill the blank
  elif existing[field] == new[field]:
    # no-op — same value
  else:
    log_conflict(field, existing[field], new[field])  # do not overwrite
```

**Result:** The note grows richer with each new source that mentions the
entity, but never becomes source-contaminated.

## Category 2: Container Identity

**Types:** organization, project

**Identity zone:** Treated as pure-atomic (same algorithm as Category 1).

**Roster sections:** Each template declares one or more `roster_sections`
(e.g., `## People` for organizations, `## Contributors` for projects).
These sections accept additive set-union merges:

```
merged_rows = set_union(existing_rows, new_rows, dedupe_by=section.dedupe_by)
```

A new row is added only if no existing row matches on all `dedupe_by` fields.
For each row added, an edge is written:
  `container_note → has_member → member_note` with `metadata: {"role": "..."}`

**Result:** The identity zone stays pure; the roster accumulates members
across sources without duplication.

## Category 3: Source-Bound

**Types:** context, event, task, action_item_list, outline

**Key:** `(source_id, toc_address)` — unique per source.

**Rule:** Write-once per source. Re-ingest of the same source with the
same content hash is a no-op. Re-ingest with a changed content hash
overwrites the node in place (`contribution_type = 'regenerated'`).

**Algorithm:**
```
existing = lookup(source_id, toc_address)
if not existing:
  create_note()                        # contribution: 'created'
elif source.content_hash == original_hash:
  return                               # no-op
else:
  overwrite_note()                     # contribution: 'regenerated'
```

**Result:** Source-bound notes are authoritative for their source. They
never merge across sources — each source gets its own context/event/task
nodes.

## Conflict Logging

When a pure-atomic or container identity field conflict is detected,
the daemon inserts a `source_contributions` row:

```
contribution_type: 'conflict'
payload: {"field": "...", "existing": "...", "new": "...", "source_new": "..."}
```

The existing value in the note file is preserved unchanged. The conflict
is visible in the DB for manual review.

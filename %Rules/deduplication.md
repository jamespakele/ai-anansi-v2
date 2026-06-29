---
id: deduplication-rules
type: rule
rule: deduplication
status: active
version: "1.0"
last_updated: 2026-06-28
---

# Deduplication Rules

Rules for detecting and resolving duplicate entities in the knowledge graph.
Applied by the Web Tender during maintenance scans and at ingest time.

## Exact Match

Two entities are considered **exact duplicates** when they share the same
`normalized_name` **and** the same `entity_type`.

**Action:** Auto-merge. The Web Tender resolves these without human review:

1. Keep the entity with the oldest `created_at` (first ingested).
2. Merge `content` fields by appending unique paragraphs from the older entity.
3. Merge `lede` and `why` fields — prefer the newer version, but keep the older
   if the newer is empty.
4. Re-point all edges from the discarded entity to the survivor.
5. Delete the discarded entity.
6. Log a `tender_queue` entry with status `resolved` and `resolved_by: 'auto'`.

## Semantic Match

Two entities are considered **semantic duplicates** when their names have a
cosine similarity **>= 0.92** (using the pgvector embedding on the `name`
field), even if the `normalized_name` differs slightly.

**Action:** Flag for review. The Web Tender creates a `tender_queue` entry with:

- `category: 'dedup'`
- `severity: 'medium'`
- `related_keys: [key_a, key_b]`
- `confidence: <cosine_similarity_score>`
- `status: 'open'`

A human (or the `web-tender-audit` skill) reviews and decides whether to merge
or dismiss.

## Merge Candidates Limit

- **Maximum merge candidates per run:** 10
- If more than 10 candidates are found in a single scan, process the 10 with
  the highest confidence scores and leave the rest for the next run.
- This prevents the tender from spending too long on dedup in a single pass.

## Conflict Resolution

When merging two entities with conflicting field values:

| Field       | Tiebreaker                                      |
|-------------|--------------------------------------------------|
| `name`      | Prefer the longer, more descriptive name         |
| `lede`      | Prefer the newer entry                           |
| `why`       | Prefer the newer entry                           |
| `content`   | Append unique paragraphs, deduplicate by hash    |
| `entity_type` | Must match — if they differ, do not auto-merge |

If `entity_type` differs between two candidates, the match is **not** a
duplicate — it is a `type_consistency` issue instead.

## Edge Cases

- **Three or more duplicates** of the same entity → merge pairwise, survivor
  accumulates all content. Log one `tender_queue` entry per pair.
- **Circular merge** (A → B, B → C, C → A) → flag as `circular` category,
  do not auto-merge.
- **Name collision across types** (e.g. "Apple" as both Organization and
  Concept) → not a duplicate. No action needed.

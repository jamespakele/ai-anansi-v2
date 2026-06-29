---
id: edge-inference-rules
type: rule
rule: edge-inference
status: active
version: "1.0"
last_updated: 2026-06-28
---

# Edge Inference Rules

Rules for automatically suggesting edges between entities when content mentions
a known entity by name or match_key. Applied by the Web Tender during
maintenance scans.

## Trigger Condition

Suggest an edge when **all** of the following are true:

1. A note's `content` or `lede` field contains a known entity's `name` or
   `match_key` as a substring or wikilink (`[[entity]]`).
2. The mentioned entity is **not already directly connected** to the source
   entity by an existing edge of the same type.
3. The inferred confidence score is **> 0.7**.

## Confidence Scoring

| Signal                                  | Weight  |
|-----------------------------------------|---------|
| Exact name match in content             | +0.30   |
| Wikilink `[[entity]]` present           | +0.40   |
| match_key appears in content            | +0.25   |
| Entity appears in `lede` (not just body) | +0.15   |
| Multiple mentions in same note          | +0.10   |
| Context keywords match edge type         | +0.10   |

Confidence is capped at **1.0**. Only suggestions with a total **> 0.7**
are written to the queue.

## Edge Type Inference

The edge type is inferred from context keywords surrounding the mention:

| Context Keywords                          | Inferred Edge Type |
|-------------------------------------------|--------------------|
| "works at", "employed by", "joined"       | `member_of`        |
| "leads", "manages", "reports to"          | `reports_to`       |
| "partnered with", "collaborates with"     | `collaborates_with`|
| "acquired", "bought", "merged with"       | `acquired`         |
| "invested in", "funded by"               | `funded_by`        |
| "part of", "belongs to", "subsidiary of"  | `part_of`          |
| "wrote", "authored", "created"            | `authored`         |
| "mentioned in", "referenced by"           | `referenced_by`    |
| "succeeded by", "replaced by"            | `succeeded_by`     |
| "related to", "associated with"           | `related_to`       |

If no context keyword matches, default to `related_to` with a 0.1 confidence
penalty.

## Queue Entry Format

Each inferred edge is written to `tender_queue` as:

- `category: 'edge_inference'`
- `severity`: `'high'` if confidence > 0.9, `'medium'` if > 0.8, `'low'` otherwise
- `match_key`: source entity match_key
- `related_keys`: [target entity match_key, inferred_edge_type]
- `confidence`: computed score
- `description`: brief explanation of the inference (e.g. "Content mentions 'John
  Smith' in context of 'works at' → suggests member_of edge to 'Acme Corp'")

## Hard Rules

- **Never** suggest an edge that already exists (same source, target, type).
- **Never** suggest a self-loop (source == target).
- **Never** auto-create edges — only queue them for review.
- If the same edge is suggested by multiple notes, create one queue entry with
  the highest confidence score and note the multiplicity in the description.

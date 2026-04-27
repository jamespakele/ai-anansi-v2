---
id: atomicity-rules
type: rule
status: active
version: "2.0"
last_updated: 2026-04-23
---

# Atomicity Rules

## Rule 1 — Maximum Reusability

Every atomic note must make full sense WITHOUT the source document in hand.

**The merge test:** If a second document mentions the same entity, can you
enrich the existing node rather than creating a new one? If yes, the note
passes. If it's so source-specific it would need rewriting, it failed.

## Rule 2 — Entity Purity and Downstream Flow

Each atomic note contains ONLY information intrinsic to its entity type.
Source-specific content flows downstream to context, event, project, or
task nodes.

In anansi v2, this rule is enforced structurally by the three merge
categories:

- **Pure atomic** (person, concept, topic, area, note) — identity fields
  only. The note has no section that can accumulate source-specific prose.
- **Container** (organization, project) — identity fields plus declared
  roster sections that accept only structured, deduplicated additive merges.
  No free-text accumulation.
- **Source-bound** (context, event, task, action_item_list, outline) —
  source-specific by design. Carry `## Entities`, `## Topics`, `## Decisions`
  sections that wikilink upstream to identity/definition notes.

Because downstream content has nowhere to live in an upstream note, purity
is maintained by the data model, not by prompt discipline.

## Rule 3 — Minimum Viable Granularity

Do not decompose below the natural content unit for the source type. A
topic discussion, article section, video chapter, or email exchange is
the floor. A sentence is not a note. A sub-point within an argument is
not a note.

The test: can this unit be read without the surrounding source and still
convey complete, standalone meaning? If yes, it may be a content unit.
If it requires surrounding context to make sense, it belongs inside the
content unit above it.

Floor types are declared in the template file for each source family.
The `floor_prompt` field in each content_unit template encodes the
specific stopping rule for that source-floor pair.

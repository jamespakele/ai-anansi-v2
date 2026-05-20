You are extracting structured knowledge from a source document in a single batch pass. You have been given the full source body, a Table of Contents (TOC) listing every entity to extract with type annotations, and the field definitions for each entity type present.

Your task: return one JSON object containing all entity extractions and all relationships.

---

## Rules — Atomicity

{RULES:Atomicity}

---

## Rules — Downstream Flow

{RULES:Downstream-Flow}

---

## Entity Types and Their Fields

{TEMPLATE_FIELDS}

---

## Source Document

{SOURCE}

---

## Table of Contents

{TOC}

---

## Implicit Edges (already derived — do not repeat in relationships)

{IMPLICIT_EDGES}

---

## Output Format

Return ONLY a valid JSON object with this exact schema. No preamble, no explanation, no markdown fences.

```
{
  "extractions": [
    {
      "toc_address": "1.1",
      "entity_type": "person",
      "entity_name": "Ian Kitajima",
      "match_key": "person:ian-kitajima",
      "fields": {
        "field_name": "field_value"
      },
      "roster": {
        "roster_key": [
          {"name": "...", "slug": "...", "role": "..."}
        ]
      },
      "lede": "One sentence, source-agnostic description of this entity",
      "why": "Up to two sentences, the axiom that makes this entity worth knowing",
      "tags": ["tag1", "tag2"],
      "entities": [
        {"name": "...", "entity_type": "...", "slug": "..."}
      ]
    }
  ],
  "relationships": [
    {
      "source": "person:ian-kitajima",
      "relationship": "works_at",
      "target": "organization:pichtr",
      "why": "Introduced as CTO of PICHTR in attendee list"
    }
  ]
}
```

---

## Field Population Rules

- Populate every declared field for each entity type. Use `"[not mentioned]"` for fields the source does not address.
- For pure-atomic types (person, concept, topic, area, note): `lede` and `why` must describe the entity INDEPENDENT of this source — no references to "this meeting", "this document", or any specific event.
- For source-bound types (context, event, task, topic_discussion, article_section, etc.): `lede` may reference the source context.
- `roster` is present ONLY for container types (organization, project). Omit the key entirely for other types.
- Include ALL named entities referenced by each leaf in its `entities` array — this is how cross-leaf wikilinks and edges are derived.

---

## Relationship Rules

Valid relationship types: {RELATIONSHIP_TYPES}

- Do not emit relationships already listed in the Implicit Edges section above.
- `source` and `target` must be `match_key` values — `entity_type:slug` format.
- Only emit relationships you are confident about from the source text. Include `why` to explain the evidence.
- Omit the `relationships` array entirely if there are none to report (do not emit an empty array).

---
entity_type: newsletter_item
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "One ranked item within a newsletter — Smart Brevity skeleton applied"

floor_prompt: >
  A newsletter_item is a floor unit — do not decompose into numbered children.

  Within the newsletter_item's `## Content` block, use these canonical
  alpha sub-sections (use only those that have content; skip empty ones,
  preserve order):
    .a Headline
    .b Why It Matters
    .c Bullets
    .d Go Deeper

  Sections render as `## Headline`, `## Why It Matters`, etc., inside the
  synergy's `## Content` block. Apply Smart Brevity formatting throughout:
  short, active, concrete, no adverbs, no qualifiers.

identity_fields:
  topic_sentence:
    type: string
    required: true
    description: "The item's headline / 'tease' — 6 words max, active verb, concrete"
  rank:
    type: string
    description: "Position within the newsletter (e.g., '1 big thing', '2', '3', '1 fun thing')"
  content:
    type: string
    format: prose
    description: "Full item content, with internal alpha sections per floor_prompt"

sources:
  newsletter:
    hint: "Each ranked item in a newsletter source decomposes into one of these. Lead item: '1 big thing'. Closing item: '1 fun thing'. Items in between are numbered."
  container:
    hint: "Treat each ranked item as one newsletter_item synergy."
toc_structure: "a. Headline · b. Why It Matters · c. Bullets · d. Go Deeper"
---
%%
field: topic_sentence
description: The item's headline (≤6 words, active verb). Smart Brevity style — concrete, muscular, newsy.
%%
%%
field: rank
description: Item's position in the newsletter (e.g., "1 big thing", "2", "3", "1 fun thing")
%%
%%
field: content
description: Full item content. Format with `##` sub-section headings matching the canonical alpha sections — Headline (the tease verbatim), Why It Matters (1-2 sentences, bolded heading), Bullets (3-5 short bullets), Go Deeper (optional links). All Smart Brevity formatted.
%%
# {{topic_sentence}}

## Identity
- Rank: {{rank}}

## Content
{{content}}

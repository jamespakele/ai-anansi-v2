---
entity_type: company_update_item
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "One ranked item within a company update — same shape as newsletter_item, leader-shaped"

floor_prompt: >
  A company_update_item is a floor unit — do not decompose into numbered children.

  Within the company_update_item's `## Content` block, use these canonical
  alpha sub-sections (use only those that have content):
    .a Headline
    .b Why It Matters
    .c Detail

  Smart Brevity formatting throughout. The leader voice should be candid
  and authentic — not lawyered or corporate.

identity_fields:
  topic_sentence:
    type: string
    required: true
    description: "The item's headline — ≤6 words, active verb"
  rank:
    type: string
    description: "Position in the update (e.g., '1 big thing', '2', '3', 'mission tie-in', '1 fun thing')"
  content:
    type: string
    format: prose
    description: "Full item content with internal alpha sections per floor_prompt"

sources:
  company_update:
    hint: "Each item in a company update decomposes into one of these. Lead item: '1 big thing'. Closing: '1 fun thing'. Mission tie-in items often surface in the middle."
  container:
    hint: "Treat each ranked item as one synergy."
toc_structure: "a. Headline · b. Why It Matters · c. Detail"
---
%%
field: topic_sentence
description: The item's headline (≤6 words, active verb, Smart Brevity style)
%%
%%
field: rank
description: Item's position (e.g., "1 big thing", "2", "3", "1 fun thing")
%%
%%
field: content
description: Full item content. Format with `##` sub-sections — Headline (verbatim), Why It Matters (1-2 sentences, bolded), Detail (the substance, bullets if applicable). Maintain leader voice — candid and authentic, not corporate.
%%
# {{topic_sentence}}

## Identity
- Rank: {{rank}}

## Content
{{content}}

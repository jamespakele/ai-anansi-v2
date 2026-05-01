---
entity_type: presentation_slide
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "One slide in a presentation — one message, ≤20 words of body text"

floor_prompt: >
  A presentation_slide is a floor unit — do not decompose into numbered children.

  Within the presentation_slide's `## Content` block, use these canonical
  alpha sub-sections (use only those that have content):
    .a Headline
    .b Body
    .c Visual

  Smart Brevity rules: one message per slide; ≤20 words of body text;
  the headline IS the takeaway, not the topic. Pictures > words.

identity_fields:
  topic_sentence:
    type: string
    required: true
    description: "The slide's takeaway headline — the message, not the topic. ≤6 words ideal, ≤sentence length max."
  slide_number:
    type: string
    description: "Slide position in the deck (e.g., '1', '7', 'final')"
  content:
    type: string
    format: prose
    description: "Slide content with internal alpha sections per floor_prompt"

sources:
  presentation:
    hint: "Each slide in a presentation source decomposes into one of these. Final slide is the ask — flag it."
  container:
    hint: "Treat each slide as one presentation_slide synergy."
---
%%
field: topic_sentence
description: The slide's takeaway headline — what the audience should know AFTER seeing the slide. NOT the topic ("Q3 Results") but the takeaway ("Q3 revenue up 18%"). ≤6 words ideal.
%%
%%
field: slide_number
description: Slide position (e.g., "1", "7", "final"). The final slide is the ask.
%%
%%
field: content
description: Slide content. Format with `##` sub-sections matching alpha labels — Headline (the takeaway), Body (≤20 words), Visual (description of the image/chart that accompanies). Smart Brevity formatted.
%%
# {{topic_sentence}}

## Identity
- Slide: {{slide_number}}

## Content
{{content}}

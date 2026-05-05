---
entity_type: speech
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "A speech / talk — spoken remarks to a live audience; atomic synergy"

floor_prompt: >
  A speech is a single atomic synergy — do not decompose into multiple notes.

  Within the speech's `## Content` block, use these canonical alpha
  sub-sections (use only those that have content; preserve order):
    .a Hook
    .b Big Thought
    .c Why It Matters
    .d Points
    .e Call to Action
    .f Close

  Smart Brevity rules: Big Thought ≤15 words; talk ≤18 minutes (TED standard);
  3-5 numbered support points. Big Thought repeats verbatim at start and close.

identity_fields:
  name:
    type: string
    required: true
    description: "Talk title — ≤6 words"
  speaker:
    type: string
    description: "Who delivered (or will deliver) the speech"
  occasion:
    type: string
    description: "Event, audience, or context"
  date:
    type: string
    description: "Delivery date (ISO YYYY-MM-DD)"
  big_thought:
    type: string
    description: "The one sentence the audience should remember (≤15 words). Smart Brevity: muscular, one-syllable preferred, would they text it to a friend?"
  content:
    type: string
    format: prose
    description: "Full speech content with internal alpha sections per floor_prompt"

sources:
  container:
    hint: "A speech arrives as a single document — outline, transcript, or notes. Treat the whole as one synergy."
  youtube_video:
    hint: "Recorded talks ingested via YouTube also fit here if the talk is the dominant content."
toc_structure: "a. Hook · b. Big Thought · c. Why It Matters · d. Points · e. Call to Action · f. Close"
---
%%
field: name
description: Talk title (≤6 words, Smart Brevity)
%%
%%
field: speaker
description: Person delivering the speech
%%
%%
field: occasion
description: Event, audience, or context for the talk
%%
%%
field: date
description: Delivery date in ISO format
%%
%%
field: big_thought
description: The single sentence the audience should remember. ≤15 words. Smart Brevity style — muscular, concrete, one-syllable preferred. Test: would the audience text it to a friend?
%%
%%
field: content
description: Full speech content. Format with `##` sub-sections — Hook (story/fact/question opening), Big Thought (verbatim), Why It Matters, Points (3-5 numbered), Call to Action, Close (Big Thought verbatim again). Smart Brevity throughout.
%%
# {{name}}

## Identity
- Speaker: {{speaker}}
- Occasion: {{occasion}}
- Date: {{date}}

## Big Thought
{{big_thought}}

## Content
{{content}}

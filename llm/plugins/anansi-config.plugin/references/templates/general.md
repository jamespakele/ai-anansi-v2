---
entity_type: general
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "1.0"
source_family: generic
description: >
  A section of unstructured prose that has no native structure — no
  chapter markers, agenda labels, slide numbers, or headers beyond
  topic shifts (transcript, essay, letter, blog post, pasted notes).
  Used when sb-compress-toc walks a generic_prose source: the TOC
  splits it into inferred thematic segments and this template is the
  floor each segment is typed as. Conversational talks and Q&A
  transcripts that arrive as youtube_video are typed under the
  canonical sibling youtube_chapter instead (its split rule covers
  "inferred thematic segment"); this one covers everything else.
floor_prompt: |
  Split the source into general leaves — one per inferred thematic
  segment; a segment ends at a clear topic shift. If the document has
  no topic shifts below the document level, emit a single leaf.

  Compress each section with the General Smart Brevity shape applied
  per-section (adapted from youtube_chapter: a. Key Points · b. Quotes ·
  c. Takeaways):
    a. Key Points   — 1-3 numbered, verb-first facts this section
                       establishes; ≤20 words each
    b. Quotes       — at most one load-bearing verbatim quote, ≤25 words
    c. Takeaway     — one sentence, ≤15 words: what a reader must carry
  If a segment has no quotable line, omit b. Do not summarize away
  named entities, numbers, or decisions mentioned in passing — sb-detailed-read
  runs a Varys pass later, but this floor is where most buried signals
  survive or die.
sources:
  generic_prose:
    hint: >
      Split the document on topic shifts when no structural markers
      exist (sb-create-toc: generic → inferred thematic segments).
      Each segment becomes one general leaf. Titles are
      synthesized (mark with * per sb-create-toc rule).
toc_structure: "a. Key Points · b. Quotes · c. Takeaway"
---

%%
field: topic_sentence
description: One sentence naming this segment's topic, as framed in the source (≤15 words)
format: prose
constraints: "Source-specific, not a generic definition — mirror the speaker's or author's framing"
%%

%%
field: key_points
description: 1-3 numbered verb-first facts this section establishes
format: bullets
%%

%%
field: quotes
description: One load-bearing verbatim quote, ≤25 words, with context
format: prose
constraints: "Omit if nothing is quotable. Attribution inline when the speaker differs from the document's primary voice"
%%

%%
field: takeaway
description: One sentence the reader must retain (≤15 words)
format: prose
%%

# {{topic_sentence}}

## a. Key Points
{{key_points}}

## b. Quotes
{{quotes}}

## c. Takeaway
{{takeaway}}
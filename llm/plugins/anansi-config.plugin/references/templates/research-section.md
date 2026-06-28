---
entity_type: article_section
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A subsection or paragraph of a research paper or article that makes one
  complete argument, claim, or point. This is the atomization floor for
  research-paper sources — do not decompose into individual sentences or
  claims.
floor_prompt: |
  Decompose this research paper into article_section leaves — one per
  distinct subsection or coherent argumentative unit. A leaf must be able
  to stand alone: a complete claim with its supporting evidence or
  reasoning. Do not create separate leaves for individual sentences,
  data points, or sub-claims within an argument. An introduction gets one
  leaf. A conclusion gets one leaf. Each numbered or headed subsection
  typically gets one leaf unless it contains two clearly independent
  arguments, in which case it may get two.
sources:
  research_paper:
    hint: >
      Capture the argument, not just the topic. The topic sentence is the
      claim being made. Supporting details are the evidence. Do not
      summarize so aggressively that the distinction between claim and
      evidence is lost.
toc_structure: "a. Key Findings · b. Methodology · c. Implications"
---

%%
field: topic_sentence
description: The central claim or point this section makes
format: prose
constraints: "One sentence. This is the argument, not just the subject."
%%

%%
field: supporting_details
description: Evidence, examples, data, or reasoning that supports the topic sentence
format: bullets
constraints: "3-7 bullets. Concrete and specific. No repetition of the topic sentence."
%%

%%
field: content
description: Full prose summary of this section preserving the author's argument
format: prose
constraints: "2-4 sentences. Maintain the argumentative structure — do not flatten into a topic list."
%%

%%
field: entities
description: Concepts, people, organizations, and references mentioned in this section
format: bullets
%%

# {{topic_sentence}}

## Supporting Details
{{supporting_details}}

## Content
{{content}}

## Entities
{{entities}}

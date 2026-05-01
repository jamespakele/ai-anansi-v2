---
entity_type: book_chapter
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "1.0"
description: >
  A chapter or named section of a nonfiction book, covering one coherent
  topic or argument. This is the atomization floor for book sources — do
  not decompose further into individual sentences or sub-claims. Each
  chapter produces a set of alpha sub-addresses containing actual Smart
  Brevity content points (not label names): key phrases followed by dense,
  specific explanations.
floor_prompt: |
  Decompose this book chapter into content-point leaves — one alpha
  address per key idea, concept, or claim. Each alpha entry must contain
  actual content in Smart Brevity format: "Key phrase — explanation"
  where the key phrase names the concept (3–6 words) and the explanation
  is dense, specific, and self-contained (one clause or short sentence).
  Do NOT use bare structural labels — never write "Summary", "Key Points",
  "Supporting Details", or any template field name as an alpha entry.
  Aim for 2–6 alphas per chapter, capturing the ideas a reader would want
  to retrieve later. Skip purely transitional or narrative content with no
  distinct, retrievable claim.
sources:
  book:
    hint: >
      Capture the idea, not the prose. The key phrase is the concept name
      or claim compressed to 3–6 words. The dash explanation is the
      specific detail — a number, a distinction, a consequence — that
      makes this point retrievable and useful without re-reading the
      chapter. Avoid restating the key phrase in the explanation.
toc_structure: "content_filled: Key phrase — explanation · 2–6 alphas per chapter"
---

%%
field: key_points
description: The 2–6 key ideas or claims in this chapter, in Smart Brevity format
format: bullets
constraints: "Each bullet: 'Key phrase — explanation'. No bare structural labels. One clause per explanation. Specific and dense."
%%

%%
field: content
description: Full prose summary of this chapter preserving the author's argument
format: prose
constraints: "2–4 sentences. Capture the main argument, the evidence or method, and why it matters."
%%

%%
field: entities
description: Concepts, people, organizations, and references mentioned in this chapter
format: bullets
%%

# {{chapter_title}}

## Key Points
{{key_points}}

## Content
{{content}}

## Entities
{{entities}}

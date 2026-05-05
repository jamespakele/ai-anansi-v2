---
entity_type: book_section
template_class: content_unit
atomic: false
merge_strategy: source_bound
template_version: "1.0"
description: >
  A named Part or Section grouping within a nonfiction book — a structural
  container that holds multiple chapters. Book sections are NOT atomized
  directly; they receive a decimal address for navigation but produce no
  alpha sub-addresses. Their chapters are the atomization leaves. Examples:
  "Part 1: The Fundamentals", "Section II: Applications".
floor_prompt: |
  This is a container section — a Part or named grouping that holds
  multiple chapters. Assign it a decimal address for navigation, but do
  NOT generate alpha sub-addresses for it. The chapters within it each
  get their own decimal sub-address (e.g. 3.2 is the section, 3.2.1 is
  the first chapter in it). If the section has a brief introduction or
  framing paragraph of its own, you may add a single alpha (3.2.a) for
  that framing — but only if the intro is substantive, not just a
  transition sentence.
sources:
  book:
    hint: >
      Use the book's own Part or Section title verbatim. Do not paraphrase.
      The section's job is navigation — the real content lives in its
      chapters.
toc_structure: "none"
---

%%
field: title
description: The section title as it appears in the book
format: prose
constraints: "Verbatim from source. Include 'Part N:' or 'Section N:' prefix if present."
%%

%%
field: scope
description: What this section covers — one sentence
format: prose
constraints: "One sentence. What is the organizing theme or question this section addresses?"
%%

# {{title}}

## Scope
{{scope}}

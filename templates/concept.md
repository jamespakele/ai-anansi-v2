---
entity_type: concept
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "A single abstract idea or principle"

identity_fields:
  name:
    type: string
    required: true
  definition:
    type: string
    description: "Precise, source-agnostic definition"
  related_concepts:
    type: string
    description: "Comma-separated related concept names"
  summary:
    type: string
    description: "One-paragraph description independent of any source"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph treatment: what this concept is, how it works, why it matters, and anything notable or non-obvious"

sources:
  meeting_summary:
    hint: "Extract the concept as defined or used in discussion, not the specific debate context"
  email_thread:
    hint: "Extract concept as introduced or referenced, not the specific email argument"
  research_paper:
    hint: "Extract formal definition or usage from the paper"
  container:
    hint: "Extract concept as defined or referenced in the document"
---
%%
field: name
description: The concept name
%%
%%
field: definition
description: Precise source-agnostic definition of this concept
%%
%%
field: related_concepts
description: Names of closely related concepts, comma-separated
%%
%%
field: summary
description: One paragraph describing this concept independent of any source
%%
%%
field: content
description: Write 2–4 paragraphs explaining this concept. Cover what it is, how it works, why it matters, and anything notable or non-obvious. Do not reference this specific source document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Definition
{{definition}}

## Summary
{{summary}}

## Related Concepts
{{related_concepts}}

## Content
{{content}}

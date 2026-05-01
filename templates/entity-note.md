---
entity_type: note
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "Fallback for untyped content that doesn't fit a more specific entity type"

identity_fields:
  name:
    type: string
    required: true
  content:
    type: string
    format: prose
    description: "The note content, source-agnostic where possible"
  summary:
    type: string
    description: "One-sentence description"

sources:
  meeting_summary:
    hint: "Use for observations or ideas that don't fit person, concept, topic, or area"
  email_thread:
    hint: "Use for unclassifiable content fragments"
  research_paper:
    hint: "Use for unclassifiable findings or observations"
  container:
    hint: "Use for unclassifiable content"
---
%%
field: name
description: Name or title of this note
%%
%%
field: content
description: The note content, kept as source-agnostic as possible
format: prose
%%
%%
field: summary
description: One-sentence description of what this note captures
%%
# {{name}}

## Summary
{{summary_5}}

## Content
{{content}}

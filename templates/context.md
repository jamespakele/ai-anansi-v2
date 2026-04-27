---
entity_type: context
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: "A unit of interaction within an event — source-specific, never reused"

identity_fields:
  name:
    type: string
    required: true
  event:
    type: string
    description: "Parent event name"
  summary:
    type: string
    description: "What happened in this interaction"
  decisions:
    type: string
    format: bullets
    description: "Decisions made"
  key_points:
    type: string
    format: bullets
    description: "Main points discussed"
  content:
    type: string
    format: prose
    description: "2–4 paragraph narrative of what happened in this interaction: who was involved, what was exchanged, what it means, and how it connects to the broader context"

sources:
  meeting_summary:
    hint: "Capture the substance of this session or agenda item — what was discussed, decided, surfaced"
  email_thread:
    hint: "Capture the key exchange content — what was communicated and decided"
  research_paper:
    hint: "Capture the key findings or argument of this section"
  container:
    hint: "Capture the substance of this document section"
---
%%
field: name
description: Name of this context node (session, agenda item, exchange, section)
%%
%%
field: event
description: Name of the parent event this context belongs to
%%
%%
field: summary
description: What happened in this interaction
%%
%%
field: decisions
description: Decisions made during this interaction
format: bullets
%%
%%
field: key_points
description: Main points discussed or surfaced
format: bullets
%%
%%
field: content
description: Write 2–4 paragraphs narrating what happened in this interaction. Cover who was involved, what was exchanged, what it means, and how it connects to the broader event or situation. You may reference this specific document as the source.
%%
# {{name}}

## Summary
{{summary}}

## Key Points
{{key_points}}

## Decisions
{{decisions}}

## Content
{{content}}

## Entities

## Topics

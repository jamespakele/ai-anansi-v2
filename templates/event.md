---
entity_type: event
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: "A project with tight time bounds (hours to days) — source-specific"

identity_fields:
  name:
    type: string
    required: true
  date:
    type: string
    description: "ISO-8601 date or date range"
  location:
    type: string
    description: "Physical or virtual location"
  organizer:
    type: string
    description: "Person or organization running the event"
  summary:
    type: string
    description: "What this event was about and what was accomplished"
  outcomes:
    type: string
    format: bullets
    description: "Key outcomes or results"
  content:
    type: string
    format: prose
    description: "2–4 paragraph narrative of what happened at this event, who was involved, what was significant, and what it produced"

sources:
  meeting_summary:
    hint: "Capture the event metadata — date, location, organizer, overall outcomes"
  email_thread:
    hint: "Capture event referenced in the thread — date, organizer, purpose"
  research_paper:
    hint: "Capture any workshop, conference, or study event referenced"
  container:
    hint: "Capture event metadata from the document"
---
%%
field: name
description: Name of the event
%%
%%
field: date
description: Date or date range (ISO-8601)
%%
%%
field: location
description: Physical location or virtual platform
%%
%%
field: organizer
description: Person or organization that organized the event
%%
%%
field: summary
description: What this event was and what was accomplished
%%
%%
field: outcomes
description: Key outcomes or results from the event
format: bullets
%%
%%
field: content
description: Write 2–4 paragraphs narrating what happened at this event. Cover who was involved, what was discussed or decided, what made it significant, and what it produced. You may reference this specific document as the source.
%%
# {{name}}

## Identity
- Date: {{date}}
- Location: {{location}}
- Organizer: {{organizer}}

## Summary
{{summary}}

## Outcomes
{{outcomes}}

## Content
{{content}}

## Participants

## Entities

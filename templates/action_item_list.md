---
entity_type: action_item_list
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: "A cohesive set of tasks from one context — source-specific"

identity_fields:
  name:
    type: string
    required: true
  context:
    type: string
    description: "The context or meeting session this list came from"
  owner:
    type: string
    description: "Person responsible for tracking this list"
  summary:
    type: string
    description: "Brief description of what this action item list covers"
  content:
    type: string
    format: prose
    description: "2–4 paragraphs explaining the context of this action item list: what situation generated it, who is involved, what the collective intent is, and what completing these items will achieve"

sources:
  meeting_summary:
    hint: "Capture the full set of action items from a session or the whole meeting"
  email_thread:
    hint: "Capture all commitments and action items from the email thread"
  research_paper:
    hint: "Capture all future work recommendations as a list"
  container:
    hint: "Capture all action items from the document"
---
%%
field: name
description: Name of this action item list
%%
%%
field: context
description: The context or meeting session this list came from
%%
%%
field: owner
description: Person responsible for tracking these items
%%
%%
field: summary
description: What this action item list covers
%%
%%
field: content
description: Write 2–4 paragraphs on the context of this action item list. Cover what situation generated it, who is involved, what the collective intent is, and what completing these items will achieve. You may reference this specific document as the source.
%%
# {{name}}

## Context
{{context}}

## Owner
{{owner}}

## Summary
{{summary}}

## Content
{{content}}

## Action Items

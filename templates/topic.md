---
entity_type: topic
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "A named subject area (no formal definition required)"

identity_fields:
  name:
    type: string
    required: true
  description:
    type: string
    description: "Brief source-agnostic description of this subject area"
  summary:
    type: string
    description: "One-paragraph overview"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph treatment: what this topic covers, why it matters, key dimensions and debates, and anything notable"

sources:
  meeting_summary:
    hint: "Extract the topic as a subject of discussion, not the specific discussion content"
  email_thread:
    hint: "Extract the subject area being addressed, not the email contents"
  research_paper:
    hint: "Extract as a field or subject area, not the paper's argument"
  container:
    hint: "Extract the subject area from the document"
---
%%
field: name
description: The topic name
%%
%%
field: description
description: Brief source-agnostic description of this subject area
%%
%%
field: summary
description: One-paragraph overview of this topic independent of any source
%%
%%
field: content
description: Write 2–4 paragraphs on this topic. Cover what it encompasses, why it matters, key dimensions or debates, and anything notable or non-obvious. Do not reference this specific source document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Description
{{description}}

## Summary
{{summary}}

## Content
{{content}}

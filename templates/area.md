---
entity_type: area
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "A domain of ongoing responsibility (no end date)"

identity_fields:
  name:
    type: string
    required: true
  owner:
    type: string
    description: "Person or role responsible"
  description:
    type: string
    description: "What this area of responsibility covers"
  summary:
    type: string
    description: "One-paragraph source-agnostic description"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph description: what this area covers, who owns it, why it matters, and its scope and boundaries"

sources:
  meeting_summary:
    hint: "Identify ongoing responsibility areas mentioned, not one-time events"
  email_thread:
    hint: "Identify ongoing domains of work or responsibility referenced"
  research_paper:
    hint: "Identify fields of study or ongoing research domains"
  container:
    hint: "Identify ongoing responsibility domains referenced"
---
%%
field: name
description: Name of the area of responsibility
%%
%%
field: owner
description: Person or role responsible for this area
%%
%%
field: description
description: What this area of responsibility covers
%%
%%
field: summary
description: One-paragraph source-agnostic description of this area
%%
%%
field: content
description: Write 2–4 paragraphs describing this area of responsibility. Cover what it encompasses, who owns it, why it matters, and its scope and boundaries. Do not reference this specific source document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Owner
{{owner}}

## Description
{{description}}

## Summary
{{summary}}

## Content
{{content}}

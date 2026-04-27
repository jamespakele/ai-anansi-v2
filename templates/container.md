---
entity_type: container
template_class: utility
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: "Generic wrapper holding multiple sub-documents — non-atomic source type, always decomposed"

identity_fields:
  name:
    type: string
    required: true
  source_type:
    type: string
    description: "The type of container (collection, report, archive, etc.)"
  summary:
    type: string
    description: "What this container holds"
  overview:
    type: string
    format: prose
    description: "The convergence narrative: what this document is, who produced it, why it exists, and what the collection of its contents represents"
  topics:
    type: string
    format: prose
    description: "Per-section breakdown of the container's contents"

sources:
  container:
    hint: "Capture the container metadata and a narrative of what it holds and why it matters. Use overview for the document's context and topics for the section-by-section breakdown."
---
%%
field: name
description: Name of the container document
%%
%%
field: source_type
description: Type of container (collection, report, archive, etc.)
%%
%%
field: summary
description: What this container holds and its overall purpose
%%
%%
field: overview
description: Write 2–4 paragraphs of context. Explain what this document is, who produced it and why, what it contains, and why it matters. Use wikilinks for named authors [[slug.person|Name]] and organizations [[slug.organization|Name]].
%%
%%
field: topics
description: Write one ### subsection per major section or document from the TOC. For each: (1) prose summarizing what that section contains and why it matters — use wikilinks for key entities [[slug.entity_type|Name]]; (2) 2–4 bullet points capturing discrete facts or findings. Example: "### Section Name\nThis section covers...\n- Key point\n- Key point"
%%
# {{name}}

## Type
{{source_type}}

## Summary
{{summary}}

## Overview
{{overview}}

## Topics
{{topics}}

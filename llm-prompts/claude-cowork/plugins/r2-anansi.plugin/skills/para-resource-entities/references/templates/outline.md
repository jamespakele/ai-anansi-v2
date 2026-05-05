---
entity_type: outline
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: "Map-of-Content for one source; renders the TOC with wikilinks to every leaf note"

identity_fields:
  name:
    type: string
    required: true
  source_id:
    type: string
    description: "UUID of the parent source record"
  source:
    type: string
    description: "Wikilink to the source file"
  toc_author:
    type: string
    description: "Provenance tag: ollama:<model>, claude-opus-4-7, manual, daemon:pass_1"
  toc_generated_at:
    type: string
    description: "ISO-8601 timestamp of TOC generation"

sources:
  meeting_summary:
    hint: "Generate a structured outline with all named entities, key concepts, participants, and context nodes"
  email_thread:
    hint: "Generate an outline of the thread's topics, participants, and key exchanges"
  research_paper:
    hint: "Generate an outline of the paper's sections, concepts, and findings"
  container:
    hint: "Generate an outline of the document's structure and named entities"
toc_structure: "none"
---
%%
field: name
description: Title of the outline (typically the source title + Outline)
%%
%%
field: toc_author
description: Who or what generated the TOC (provenance tag)
%%
# {{name}}

[[-Outline]]

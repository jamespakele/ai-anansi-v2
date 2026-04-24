---
entity_type: container
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

sources:
  container:
    hint: "Identify the overarching container — what it is and what it holds. The sub-documents are its real content."
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
# {{name}}

## Type
{{source_type}}

## Summary
{{summary}}

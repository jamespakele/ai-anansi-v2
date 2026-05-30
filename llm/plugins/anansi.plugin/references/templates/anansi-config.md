---
entity_type: anansi_config
template_class: utility
atomic: true
merge_strategy: pure_atomic
template_version: "1.0"
description: "Runtime configuration entry for the anansi knowledge graph — stores template definitions, edge vocabularies, processing rules, and other config that can be updated at runtime without server restart"

identity_fields:
  name:
    type: string
    required: true
    description: "Human-readable config entry name (e.g., 'Template: person', 'Edge Types')"
  config_type:
    type: string
    required: true
    description: "Category of config — 'template', 'edge_vocabulary', 'processing_rule', 'preference'"
  config_version:
    type: string
    description: "Version of this config entry (e.g., '3.0' for a template version)"

sources:
  generic_prose:
    hint: "Config entries are created programmatically by skills or tools, not extracted from prose sources."
toc_structure: "none"
---
%%
field: name
description: Human-readable name for this config entry. For template definitions, use 'Template: {entity_type}'.
%%
%%
field: config_type
description: Category — 'template' for entity type template definitions, 'edge_vocabulary' for relationship type lists, 'processing_rule' for pipeline behavior, 'preference' for user settings.
%%
%%
field: config_version
description: Version string for this config entry. For templates, matches template_version from frontmatter.
%%
# {{name}}

## Config Type
{{config_type}}

## Version
{{config_version}}

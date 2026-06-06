---
entity_type: manifest
template_class: utility
atomic: true
merge_strategy: container
template_version: "1.0"
description: "Standing Map-of-Content for one PARA band (projects or areas). A numbered outline that accretes entries over time as projects/areas are registered — each entry edged to its entity node via contains/part_of. Unlike `outline` (source_bound, born from one ingestion), a manifest is container-merge: it grows across the life of the vault, never frozen to a single source."

identity_fields:
  name:
    type: string
    required: true
    description: "Manifest title, e.g. 'Projects Manifest' or 'Areas Manifest'"
  band:
    type: string
    description: "Which PARA band this manifest indexes: 'projects' or 'areas'. Drives the section heading and the entity_type it edges to."
  generated_at:
    type: string
    description: "ISO-8601 timestamp of the last reconcile/update"

roster_sections:
  entries:
    source_field: entries
    render_as: "## 1. Entries"
    row_format: "- {address} [[-{slug}|{name}]]"
    dedupe_by: [slug]

sources:
  container:
    hint: "Not produced by atomization — a manifest is maintained by the pm-konohiki skills (pm-review / pm-new-project / pm-new-area), which append entries and wire contains/part_of edges to entity nodes."
toc_structure: "none"
---
%%
field: name
description: Title of the manifest (e.g. "Projects Manifest")
%%
%%
field: band
description: PARA band this manifest indexes — "projects" or "areas"
%%
%%
field: generated_at
description: ISO-8601 timestamp of the last reconcile

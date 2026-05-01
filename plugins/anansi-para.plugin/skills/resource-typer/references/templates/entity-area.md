---
entity_type: area
template_class: identity
atomic: true
merge_strategy: container
template_version: "3.0"
description: "A domain of ongoing responsibility (no end date) — PARA Area"

atomic_criteria: >
  Represents one ongoing responsibility domain. Same shape as project
  (active synergy), just without a target end date. Stakeholders
  accumulate additively over time (people who have been part of this).

identity_fields:
  name:
    type: string
    required: true
  owner:
    type: string
    description: "Person or role primarily responsible"
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

roster_sections:
  stakeholders:
    source_field: stakeholders
    render_as: "## Stakeholders"
    row_format: "- [[-{slug}|{name}]] — {role}"
    dedupe_by: [slug, role]

sources:
  meeting_summary:
    hint: "Identify ongoing responsibility areas mentioned, not one-time events"
  email_thread:
    hint: "Identify ongoing domains of work or responsibility referenced"
  research_paper:
    hint: "Identify fields of study or ongoing research domains"
  container:
    hint: "Identify ongoing responsibility domains referenced"
toc_structure: "none"
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
description: One-paragraph source-agnostic de
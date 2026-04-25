---
entity_type: organization
atomic: true
merge_strategy: container
template_version: "2.0"
description: "A named group — company, ngo, government body, team"

atomic_criteria: >
  Represents one real-world organization with durable identity. Cannot be
  split without losing coherence. Personal roles and event-specific
  activities belong downstream.

identity_fields:
  name:
    type: string
    required: true
    description: "Primary org name as commonly used"
  full_name:
    type: string
    description: "Expanded or legal name if different"
  type:
    type: string
    description: "Company, NGO, government agency, research institution, etc."
  domain:
    type: string
    description: "Field of operation"
  summary:
    type: string
    description: "One-paragraph source-agnostic description"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph profile: what this organization does, its mission, significance, structure, and anything notable or non-obvious"

roster_sections:
  people:
    source_field: people
    render_as: "## People"
    row_format: "- [[-{slug}|{name}]] — {role}"
    dedupe_by: [slug, role]

sources:
  meeting_summary:
    hint: "Check org name in intros, attendee list, 'from {org}' attributions"
  email_thread:
    hint: "Check From domain, signature blocks, company mentions"
  research_paper:
    hint: "Check author affiliations, funder acknowledgments, cited institutions"
  container:
    hint: "Check any named organizations or groups referenced."
---
%%
field: name
description: The primary organization name
%%
%%
field: full_name
description: Expanded or legal name if different from primary
%%
%%
field: type
description: Category of organization (company, NGO, government, etc.)
%%
%%
field: domain
description: Primary field of operation or focus area
%%
%%
field: summary
description: One-paragraph source-agnostic description
%%
%%
field: content
description: Write 2–4 paragraphs profiling this organization. Cover what it does, its mission and significance, how it operates, and anything notable or non-obvious. Do not reference this specific document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Identity
- Full name: {{full_name}}
- Type: {{type}}
- Domain: {{domain}}

## Summary
{{summary}}

## Content
{{content}}

## People

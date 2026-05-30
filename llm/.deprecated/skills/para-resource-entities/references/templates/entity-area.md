---
entity_type: area
template_class: identity
atomic: true
merge_strategy: container
template_version: "3.1"
description: "A domain of ongoing responsibility (no end date) — PARA Area"

atomic_criteria: >
  Represents one ongoing responsibility domain. Per Forte (PARA Method
  Ch. 3): an Area has a standard to maintain AND an indefinite end date
  AND direct responsibility from the user. Same shape as project (active
  synergy), just without a target end date. Stakeholders accumulate
  additively over time (people who have been part of this).

identity_fields:
  name:
    type: string
    required: true
  standard:
    type: string
    required: true
    description: "The quality bar the user is committed to upholding indefinitely — Forte's defining trait of an Area (PARA Method, Ch. 3: 'A standard to be maintained'). Examples: 'pay all bills on time and provide for family's needs' (Finances); 'spend quality time with kids every evening' (Parenting); 'upgrade speed/performance, fix bugs quickly, approve new releases' (Product Development)."
  owner:
    type: string
    description: "Person or role primarily responsible"
  description:
    type: string
    description: "What this area of responsibility covers — its scope and subject. Distinct from `standard`, which is the quality bar; description is the territory the standard applies to."
  summary:
    type: string
    description: "One-paragraph source-agnostic description"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph description: what this area covers, who owns it, what standard is being upheld, why it matters, and its scope and boundaries"

roster_sections:
  stakeholders:
    source_field: stakeholders
    render_as: "## Stakeholders"
    row_format: "- [[-{slug}|{name}]] — {role}"
    dedupe_by: [slug, role]

sources:
  email:
    hint: "Identify ongoing domains of responsibility referenced — recurring topics, maintained relationships, role-based ownership"
  email_thread:
    hint: "Identify ongoing domains of work or responsibility referenced across the thread"
  newsletter:
    hint: "Identify ongoing responsibility domains the user maintains, not one-time launches"
  meeting:
    hint: "Identify ongoing responsibility areas mentioned, not one-time events"
  presentation:
    hint: "Identify ongoing domains the slides describe the user as responsible for"
  speech:
    hint: "Identify ongoing responsibility domains the speaker presents themselves as maintaining"
  social_media:
    hint: "Identify ongoing responsibility domains the user references as their own (rare in this format)"
  workplace_memo:
    hint: "Identify named ongoing responsibilities — roles, account management, recurring duties"
  company_update:
    hint: "Identify ongoing responsibility domains owned by the user — typically named functions or roles"
  book:
    hint: "Identify fields of study or ongoing research domains the user maintains"
  generic_prose:
    hint: "Identify ongoing responsibility domains referenced — roles, hats worn, standards upheld"
toc_structure: "none"
---
%%
field: name
description: Name of the area of responsibility
%%
%%
field: standard
description: The quality bar the user is committed to upholding in this area indefinitely. Forte's defining trait of an Area (PARA Method, Ch. 3). Stated in the user's own words from the source when possible.
%%
%%
field: owner
description: Person or role responsible for this area
%%
%%
field: description
description: What this area of responsibility covers — the scope and subject. Distinct from `standard` (the quality bar).
%%
%%
field: summary
description: One-paragraph source-agnostic description of the area
%%
%%
field: content
description: Write 2–4 paragraphs on this area. Cover what it is, who owns it, what standard is being upheld, why it matters, and its scope and boundaries. Do not reference this specific document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Identity
- Standard: {{standard}}
- Owner: {{owner}}
- Description: {{description}}

## Summary
{{summary}}

## Content
{{content}}

## Stakeholders

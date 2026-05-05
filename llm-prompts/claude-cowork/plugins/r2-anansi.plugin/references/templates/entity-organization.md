---
entity_type: organization
template_class: identity
atomic: true
merge_strategy: container
template_version: "3.0"
description: "A named group — company, NGO, government body, team, agency"

atomic_criteria: >
  Represents one real-world organization with durable identity. No narrative
  summary or description in the body. Membership is tracked via edges
  (member_of, employed_by, led_by) — not in the body — because membership
  churns. What stays in the body: identity fields (name, type, domain) plus
  an additive `## Context` section of orphaned facts that don't deserve
  their own atomic note.

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

roster_sections:
  context:
    source_field: context
    render_as: "## Context"
    row_format: "- {text}"
    dedupe_by: [text]
    description: >
      Running bulleted list of orphaned facts about this organization —
      things mentioned in passing in source documents that don't deserve
      their own atomic note. Each bullet must use Smart Brevity style:
      short, active, concrete, no adverbs, no qualifiers. One fact per
      bullet. Examples:
      "- donated $250K to DCS in 2024"
      "- moved HQ from Phoenix to Austin in 2023"
      "- raised Series B led by Sequoia Q3 2025"
      "- sponsored the 2024 Hawaii AI Summit"
      The list grows additively — old facts stay; new ingests append new
      bullets. Bullets are deduplicated by exact text match.

sources:
  meeting_summary:
    hint: "Check org name in intros, attendee list, 'from {org}' attributions. Add new facts (donations, moves, hires-as-org-action, milestones) to context."
  email_thread:
    hint: "Check From domain, signature blocks, company mentions. Add notable facts to context."
  research_paper:
    hint: "Check author affiliations, funder acknowledgments, cited institutions. Add funder/affiliation facts to context."
  container:
    hint: "Check named organizations referenced. Add notable facts to context."
toc_structure: "none"
---
%%
field: name
description: The primary organization name as commonly used
%%
%%
field: full_name
description: Expanded or legal name if different from primary
%%
%%
field: type
description: Category of organization — company, NGO, government, research institution, etc.
%%
%%
field: domain
description: Primary field of operation or focus area
%%
# {{name}}

## Identity
- Full name: {{full_name}}
- Type: {{type}}
- Domain: {{domain}}

## Context

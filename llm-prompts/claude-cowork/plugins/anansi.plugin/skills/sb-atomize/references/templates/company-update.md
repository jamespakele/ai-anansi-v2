---
entity_type: company_update
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "3.0"
description: "A leader-to-organization update — CEO/exec to whole org, recurring cadence"

identity_fields:
  name:
    type: string
    required: true
    description: "Issue title (e.g., '5 Big Things — Week of April 28')"
  publication:
    type: string
    description: "Recurring newsletter name (e.g., '5 Big Things', 'CEO Weekly')"
  date:
    type: string
    description: "Publication date (ISO YYYY-MM-DD)"
  author:
    type: string
    description: "Leader / executive author"
  audience:
    type: string
    description: "Recipient scope (whole company, business unit, exec team, etc.)"
  overview:
    type: string
    format: prose
    description: "One-paragraph framing of what this issue covers"

sources:
  container:
    hint: "Company updates arrive as one document. Decompose into company-update-item synergies. The 'mission tie-in' bullet (per Smart Brevity Ch. 21) often appears as one item; '1 fun thing' often closes."
toc_structure: "none"
---
%%
field: name
description: The issue's title
%%
%%
field: publication
description: Name of the recurring leader-newsletter
%%
%%
field: date
description: Publication date in ISO format
%%
%%
field: author
description: Leader or executive author
%%
%%
field: audience
description: Whose inbox this lands in (whole company, BU, etc.)
%%
%%
field: overview
description: One-paragraph framing — what the leader is trying to align the org on this week
%%
# {{name}}

## Identity
- Publication: {{publication}}
- Date: {{date}}
- Author: {{author}}
- Audience: {{audience}}

## Overview
{{overview}}

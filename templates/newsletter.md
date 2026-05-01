---
entity_type: newsletter
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "3.0"
description: "A newsletter — recurring publication with multiple ranked items"

identity_fields:
  name:
    type: string
    required: true
    description: "Issue title or subject line"
  publication:
    type: string
    description: "Name of the recurring newsletter (e.g., Axios AM, 5 Big Things)"
  date:
    type: string
    description: "Publication date (ISO YYYY-MM-DD)"
  author:
    type: string
    description: "Author or editor of the issue"
  overview:
    type: string
    format: prose
    description: "One-paragraph overview of the issue's themes"

sources:
  container:
    hint: "Newsletters arrive as a single document; treat the whole as the source. Decomposes into newsletter-item synergies, with the lead item labeled '1 big thing' and the closing item labeled '1 fun thing'."
---
%%
field: name
description: The issue's title or subject line (e.g., "Axios AM 2026-04-29")
%%
%%
field: publication
description: Name of the recurring publication
%%
%%
field: date
description: Publication date in ISO format (YYYY-MM-DD)
%%
%%
field: author
description: Author or editor of the issue
%%
%%
field: overview
description: One-paragraph overview describing the issue's themes and content arc
%%
# {{name}}

## Identity
- Publication: {{publication}}
- Date: {{date}}
- Author: {{author}}

## Overview
{{overview}}

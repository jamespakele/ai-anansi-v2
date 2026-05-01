---
entity_type: presentation
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "3.0"
description: "A presentation / slide deck — visual-verbal communication"

identity_fields:
  name:
    type: string
    required: true
    description: "Deck title"
  presenter:
    type: string
    description: "Person delivering the presentation"
  date:
    type: string
    description: "Presentation date or scheduled date (ISO YYYY-MM-DD)"
  audience:
    type: string
    description: "Intended audience (board, customers, all-hands, etc.)"
  ask:
    type: string
    description: "The presentation's outcome — what the presenter wants the audience to do or believe (Smart Brevity: 6 words max for the ask)"
  overview:
    type: string
    format: prose
    description: "One-paragraph overview of the deck's argument arc"

sources:
  container:
    hint: "A deck arrives as a single source. Decomposes into presentation-slide synergies, one per slide. Smart Brevity recommends 5-12 slides, dozen max. The final slide is the ask."
---
%%
field: name
description: The deck's title
%%
%%
field: presenter
description: Person delivering the presentation
%%
%%
field: date
description: Presentation date in ISO format (YYYY-MM-DD)
%%
%%
field: audience
description: Intended audience (board, customers, all-hands, etc.)
%%
%%
field: ask
description: The deck's ultimate outcome — what the presenter wants the audience to do or believe. ≤6 words, Smart Brevity style.
%%
%%
field: overview
description: One-paragraph overview describing the deck's argument arc and structure
%%
# {{name}}

## Identity
- Presenter: {{presenter}}
- Date: {{date}}
- Audience: {{audience}}
- Ask: {{ask}}

## Overview
{{overview}}

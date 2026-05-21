---
entity_type: meeting_summary
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: "A meeting record — non-atomic source type, always decomposed into atomic notes"

identity_fields:
  name:
    type: string
    required: true
  date:
    type: string
    description: "ISO-8601 date"
  location:
    type: string
    description: "Physical or virtual location"
  facilitator:
    type: string
    description: "Person who ran the meeting"
  attendees:
    type: string
    description: "Comma-separated list of attendees"
  summary:
    type: string
    description: "High-level summary of what was discussed and decided"
  overview:
    type: string
    format: prose
    description: "The full convergence narrative: who gathered, why these specific people and organizations came together, what the meeting was trying to accomplish, and what it means in the broader context"
  topics:
    type: string
    format: prose
    description: "Per-topic sections covering the substance of the meeting"

sources:
  meeting_summary:
    hint: "Capture the meeting metadata and the full narrative of what happened. Use overview for the connective tissue and topics for the per-agenda-item breakdown."
toc_structure: "none"
---
%%
field: name
description: Meeting name or title
%%
%%
field: date
description: Date of the meeting (ISO-8601)
%%
%%
field: location
description: Physical location or virtual platform
%%
%%
field: facilitator
description: Person who facilitated or ran the meeting
%%
%%
field: attendees
description: All attendees, comma-separated
%%
%%
field: summary
description: High-level summary of what was discussed and decided
%%
%%
field: overview
description: Write 2–4 paragraphs of connective tissue. Explain who gathered and why — the specific combination of people, organizations, and topics that made this meeting happen. What brought them together? What were they trying to achieve? What does this convergence mean? Use wikilinks for named participants and organizations: [[slug.entity_type|Display Name]].
%%
%%
field: topics
description: Write one ### subsection per major topic or agenda item from the TOC. For each topic: (1) prose explaining who was involved in that part of the conversation and what was exchanged — name specific participants using wikilinks [[slug.person|Name]] and organizations [[slug.organization|Name]]; (2) 2–4 bullet points capturing discrete takeaways, decisions, or action items. Keep the wikilink slugs lowercase-hyphenated. Example subsection format: "### Topic Name\n[[person-slug.person|Person Name]] and [[org-slug.organization|Org Name]] discussed X...\n- Bullet takeaway\n- Bullet takeaway"
%%
# {{name}}

## Meeting Details
- Date: {{date}}
- Location: {{location}}
- Facilitator: {{facilitator}}
- Attendees: {{attendees}}

## Overview
{{overview}}

## Topics
{{topics}}

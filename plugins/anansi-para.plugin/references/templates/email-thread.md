---
entity_type: email_thread
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: "An email thread — non-atomic source type, always decomposed into atomic notes"

identity_fields:
  name:
    type: string
    required: true
  subject:
    type: string
    description: "Email subject line"
  participants:
    type: string
    description: "Comma-separated list of participants"
  date_range:
    type: string
    description: "Date range of the thread"
  summary:
    type: string
    description: "What the thread is about and what was resolved"
  overview:
    type: string
    format: prose
    description: "The convergence narrative: who is in this thread, why they are corresponding, what they are trying to resolve or achieve, and what the thread reveals about their relationships"
  topics:
    type: string
    format: prose
    description: "Per-topic sections covering the key exchanges in the thread"

sources:
  email_thread:
    hint: "Capture the thread metadata and the full narrative of the exchange. Use overview for why these people are corresponding and topics for the key exchanges and decisions."
toc_structure: "none"
---
%%
field: name
description: Thread name (typically derived from subject line)
%%
%%
field: subject
description: Email subject line
%%
%%
field: participants
description: All participants in the thread, comma-separated
%%
%%
field: date_range
description: Date range of the email thread
%%
%%
field: summary
description: What the thread was about and what was resolved or decided
%%
%%
field: overview
description: Write 2–4 paragraphs of connective tissue. Explain who is in this thread and why — the specific combination of people and organizations, what they are trying to resolve or achieve, and what their correspondence reveals about their relationships or interests. Use wikilinks for named participants: [[slug.entity_type|Display Name]].
%%
%%
field: topics
description: Write one ### subsection per major topic or exchange from the TOC. For each: (1) prose explaining who said what and why it matters — use wikilinks [[slug.person|Name]] and [[slug.organization|Name]]; (2) 2–4 bullet points capturing discrete decisions, commitments, or key points. Example: "### Topic Name\n[[person-slug.person|Person]] argued that...\n- Key point\n- Decision made"
%%
# {{name}}

## Thread Details
- Subject: {{subject}}
- Participants: {{participants}}
- Dates: {{date_range}}

## Overview
{{overview}}

## Topics
{{topics}}

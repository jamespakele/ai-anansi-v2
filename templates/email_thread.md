---
entity_type: email_thread
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

sources:
  email_thread:
    hint: "Capture the thread metadata: subject, participants, date range, and overall summary of what was discussed and decided"
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
# {{name}}

## Thread Details
- Subject: {{subject}}
- Participants: {{participants}}
- Dates: {{date_range}}

## Summary
{{summary}}

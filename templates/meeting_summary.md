---
entity_type: meeting_summary
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

sources:
  meeting_summary:
    hint: "Capture the meeting metadata: date, location, facilitator, attendees, and overall summary. The agenda items are context nodes."
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
# {{name}}

## Meeting Details
- Date: {{date}}
- Location: {{location}}
- Facilitator: {{facilitator}}
- Attendees: {{attendees}}

## Summary
{{summary}}

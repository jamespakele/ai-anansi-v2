---
entity_type: person
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "3.0"
description: "A unique individual — identity, contact, and connection origin"

atomic_criteria: >
  Represents one human being. Identity-only — name, contact info, and how
  the user came to know them. No narrative summaries or descriptions in
  the body. What they did, said, or contributed lives downstream in
  context/event/discussion notes; what they're affiliated with lives in
  edges (member_of, employed_by, led_by, etc.).

identity_fields:
  name:
    type: string
    required: true
  contact_email:
    type: string
    format: email
  contact_phone:
    type: string
    format: phone
  met_via:
    type: string
    description: "How the user met or came to know this person. One short sentence — naming an introducing person, an event, a project, or other connection origin. Helps the user remember the relationship's origin."

sources:
  meeting_summary:
    hint: "Check attendee list, speaker attributions, intros — but do NOT capture what they said. Extract contact_email and contact_phone if present in attendee list or signature block. Infer met_via from the meeting context (e.g., 'met at [meeting name]')."
  email_thread:
    hint: "Check From, To, and CC headers for contact_email addresses. Check signature blocks for contact_phone, title, and org. Extract email addresses directly from headers — do not infer. Infer met_via from thread context (e.g., 'via [project] email thread' or 'introduced by [person]'). Do NOT capture email body content as biographical description."
  research_paper:
    hint: "Check author byline, affiliations, and corresponding-author footnotes for contact_email. Do not capture paper content."
  container:
    hint: "Check listed participants or named individuals. Extract any contact details (email, phone) listed alongside names."
toc_structure: "none"
---
%%
field: name
description: Full name of the person
%%
%%
field: contact_email
description: Email address if mentioned in source
%%
%%
field: contact_phone
description: Phone number if mentioned in source
%%
%%
field: met_via
description: One short sentence on how the user met or came to know this person — name an introducing person, an event, a project, a mutual context. Smart-brevity style (active, concrete, ≤15 words). Helps recall the connection origin.
%%
# {{name}}

## Contact
- Email: {{contact_email}}
- Phone: {{contact_phone}}

## Met via
{{met_via}}

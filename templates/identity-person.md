---
entity_type: person
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "A unique individual"

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
  summary:
    type: string
    description: "One-sentence source-agnostic description of who this person is"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph profile: who this person is, their background, role, significance, and anything notable or non-obvious"

sources:
  meeting_summary:
    hint: "Check attendee list, speaker attributions, intros — but do NOT capture what they said"
  email_thread:
    hint: "Check From headers, signature blocks. Do NOT capture email body content in the person note."
  research_paper:
    hint: "Check author byline and affiliations. Do not capture paper content."
  container:
    hint: "Check any listed participants or named individuals."
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
field: summary
description: One-sentence description — who is this person, independent of this source
%%
%%
field: content
description: Write 2–4 paragraphs profiling this person. Cover who they are, their background and role, their significance, and anything notable or non-obvious. Do not reference this specific source document — write as a durable, source-agnostic knowledge entry that will be enriched over time.
%%
# {{name}}

## Contact
- Email: {{contact_email}}
- Phone: {{contact_phone}}

## Summary
{{summary}}

## Content
{{content}}

---
entity_type: memo
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "A workplace memo — internal one-to-many communication; atomic synergy (no decomposition)"

floor_prompt: >
  A memo is a single atomic synergy — do not decompose into multiple notes.

  Within the memo's `## Content` block, use these canonical alpha sub-sections
  (use only those that have content; preserve order):
    .a Subject
    .b Why It Matters
    .c What's Changing
    .d What's Next

  Smart Brevity formatting throughout — short, direct, scannable. Title is
  the subject (≤6 words). Bullets for any 3+ supporting points.

identity_fields:
  name:
    type: string
    required: true
    description: "Memo subject — ≤6 words, action-shaped"
  sender:
    type: string
    description: "Who sent the memo"
  recipients:
    type: string
    description: "Who received it (team, all-hands, specific people)"
  date:
    type: string
    description: "Date sent (ISO YYYY-MM-DD)"
  content:
    type: string
    format: prose
    description: "Full memo content with internal alpha sections per floor_prompt"

sources:
  container:
    hint: "Memos arrive as a single document. Treat the whole as one synergy. Subject line becomes the name."
  email_thread:
    hint: "Long-form one-to-many emails labeled as memos qualify here."
toc_structure: "a. Subject · b. Why It Matters · c. What's Changing · d. What's Next"
---
%%
field: name
description: Memo subject (≤6 words, action-shaped). Smart Brevity style.
%%
%%
field: sender
description: Who sent the memo
%%
%%
field: recipients
description: Who received the memo (team, all-hands, named people)
%%
%%
field: date
description: Date sent in ISO format
%%
%%
field: content
description: Full memo content. Format with `##` sub-sections — Subject (the news), Why It Matters (1-2 sentences, bolded), What's Changing (bullets), What's Next (owners + dates, bolded owners). Smart Brevity formatted.
%%
# {{name}}

## Identity
- From: {{sender}}
- To: {{recipients}}
- Date: {{date}}

## Content
{{content}}

---
entity_type: email_exchange
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A cohesive exchange within an email thread organized around one question,
  decision, or topic. This is the atomization floor for email-thread
  sources.
floor_prompt: |
  Decompose this email thread into email_exchange leaves — one per
  coherent exchange around a distinct question, decision, or topic. An
  exchange ends when the thread moves to a new subject or a decision
  is reached. Do not create separate leaves for individual emails within
  an exchange. A back-and-forth negotiation about one decision is one
  leaf. A thread that pivots to a new topic partway through gets a new
  leaf at the pivot point.
sources:
  email_thread:
    hint: >
      Capture what was being resolved, not who sent what when. The
      participants are who was in the exchange. The outcome is what
      was decided or agreed.
toc_structure: "a. Context · b. Decisions · c. Action Items"
---

%%
field: subject
description: What question or topic this exchange is about
format: prose
%%

%%
field: participants
description: People involved in this specific exchange
format: bullets
%%

%%
field: content
description: Summary of the exchange — what was being discussed and how it unfolded
format: prose
%%

%%
field: outcome
description: The decision, agreement, or resolution reached in this exchange
format: prose
constraints: "If unresolved, write '[unresolved — thread continues]'"
%%

# {{subject}}

## Participants
{{participants}}

## Content
{{content}}

## Outcome
{{outcome}}

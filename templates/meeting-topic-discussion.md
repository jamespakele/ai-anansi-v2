---
entity_type: topic_discussion
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A bounded discussion of a single topic within a meeting. Captures who
  participated, what was said, what was decided, and what actions followed.
  This is the atomization floor for meeting-summary sources — do not
  decompose further.
floor_prompt: |
  Decompose this meeting summary into topic_discussion leaves — one per
  distinct topic of discussion. Do not create sub-leaves within a topic
  discussion. Individual statements, speaker turns, and sub-points belong
  inside the leaf as content, not as separate TOC entries. If two agenda
  items blur together in the source, create one leaf. A side comment that
  launches a new direction is a new leaf only if it constitutes a distinct
  topic with its own participants and outcome.
sources:
  meeting_summary:
    hint: >
      Extract what was discussed, not a verbatim transcript. One sentence
      per substantive point. Capture decisions and action items precisely —
      these are the high-value outputs. Participants are people who spoke
      or were directly addressed, not everyone in the room.
---

%%
field: topic_sentence
description: One sentence naming the topic and framing what was being discussed
format: prose
constraints: "Source-specific — name the topic as it was framed in this meeting, not a generic definition"
%%

%%
field: participants
description: People who actively participated in this specific discussion
format: bullets
constraints: "Only people who spoke or were directly addressed in this topic, not all meeting attendees"
%%

%%
field: organizations
description: Organizations mentioned or represented in this discussion
format: bullets
%%

%%
field: content
description: Summary of what was discussed — the substance of the conversation
format: prose
constraints: "3-6 sentences. What was said, not who said it. No verbatim quotes."
%%

%%
field: decisions
description: Decisions reached or agreements made during this discussion
format: bullets
constraints: "Concrete outcomes only. If no decision was reached, write '[no decision reached]'"
%%

%%
field: action_items
description: Specific commitments and next steps from this discussion, with owners where known
format: bullets
constraints: "Format: 'Owner to do X by date' where information is available"
%%

# {{topic_sentence}}

## Participants
{{participants}}

## Organizations
{{organizations}}

## Content
{{content}}

## Decisions
{{decisions}}

## Action Items
{{action_items}}

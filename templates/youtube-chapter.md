---
entity_type: youtube_chapter
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A chapter or inferred thematic segment of a YouTube video or recorded
  presentation. This is the atomization floor for youtube-video sources —
  do not decompose into individual statements or timestamps.
floor_prompt: |
  Decompose this video transcript into youtube_chapter leaves — one per
  explicit chapter marker or inferred thematic segment. A segment ends
  when the speaker clearly transitions to a new topic or the content
  shifts focus. Do not create leaves for individual statements, examples,
  or asides within a segment. A brief tangent that returns to the main
  thread is content inside the current leaf, not a new leaf. Aim for
  5-15 leaves for a standard one-hour video — fewer for tightly focused
  content, more for wide-ranging discussions.
sources:
  youtube_video:
    hint: >
      Capture the substance of what was presented in this segment, not a
      transcript summary. What was the key point? What examples or
      evidence were given? What entities were mentioned?
---

%%
field: topic_sentence
description: One sentence describing what this chapter or segment is about
format: prose
%%

%%
field: key_points
description: The main points, arguments, or demonstrations in this segment
format: bullets
constraints: "3-7 bullets. Concrete and specific."
%%

%%
field: content
description: Prose summary of what was covered in this segment
format: prose
constraints: "2-4 sentences."
%%

%%
field: entities
description: People, organizations, concepts, and tools mentioned in this segment
format: bullets
%%

# {{topic_sentence}}

## Key Points
{{key_points}}

## Content
{{content}}

## Entities
{{entities}}

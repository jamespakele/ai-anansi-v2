---
entity_type: youtube_video
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: >
  A YouTube video or recorded presentation. Non-atomic source type —
  decomposes into youtube_chapter floor nodes.
---

%%
field: overview
description: >
  Convergence narrative — who made this video, why, what it is trying to
  convey, and what context it sits in. Who is the intended audience.
format: prose
%%

%%
field: topics
description: >
  Per-chapter breakdown using ### subsections, one per youtube_chapter
  leaf. Each subsection contains wikilinks to entities mentioned in
  that chapter.
format: prose
%%

# {{title}}

## Overview
{{overview}}

## Topics
{{topics}}

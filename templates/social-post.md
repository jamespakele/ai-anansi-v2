---
entity_type: social_post
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "3.0"
description: "A social media post — atomic synergy, no decomposition"

floor_prompt: >
  A social_post is a single atomic synergy. Posts are short by definition;
  most don't need internal alpha sections. For longer posts (LinkedIn-shaped,
  >100 words), optionally use:
    .a Hook
    .b Body
  The hook is the first 2 lines (pre-fold on most platforms). Apply
  Smart Brevity throughout — first 6 words decide.

identity_fields:
  topic_sentence:
    type: string
    required: true
    description: "The post's hook / first line — what makes someone stop scrolling"
  platform:
    type: string
    description: "Platform — Twitter/X, LinkedIn, Instagram, Facebook, Threads, etc."
  author:
    type: string
    description: "Who posted"
  posted_at:
    type: string
    description: "When the post went live (ISO timestamp or date)"
  url:
    type: string
    description: "Direct link to the post if available"
  content:
    type: string
    format: prose
    description: "Full post text including any hashtags/links/mentions"

sources:
  container:
    hint: "Single posts qualify as one synergy. For multi-post threads on Twitter/X, treat the thread as a source decomposing into per-post synergies (use existing email-thread pattern as a model — TBD if a separate template is warranted)."
---
%%
field: topic_sentence
description: The post's hook (first line). What makes someone stop scrolling. Smart Brevity — first 6 words decide.
%%
%%
field: platform
description: Platform name (Twitter/X, LinkedIn, Instagram, Facebook, Threads, etc.)
%%
%%
field: author
description: Who posted
%%
%%
field: posted_at
description: When the post went live (ISO timestamp or date)
%%
%%
field: url
description: Direct link to the post (if available)
%%
%%
field: content
description: Full post text — including hashtags, mentions, links. For longer posts, optionally format with `##` sub-sections matching alpha labels (Hook, Body).
%%
# {{topic_sentence}}

## Identity
- Platform: {{platform}}
- Author: {{author}}
- Posted: {{posted_at}}
- URL: {{url}}

## Content
{{content}}

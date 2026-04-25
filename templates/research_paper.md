---
entity_type: research_paper
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: "An academic or research document — non-atomic source type, always decomposed"

identity_fields:
  name:
    type: string
    required: true
  authors:
    type: string
    description: "Author names, comma-separated"
  publication_date:
    type: string
    description: "Publication date (ISO-8601)"
  venue:
    type: string
    description: "Journal, conference, or publication venue"
  abstract_summary:
    type: string
    description: "What this paper argues or demonstrates"
  overview:
    type: string
    format: prose
    description: "The convergence narrative: what question this paper addresses, who wrote it, what field it sits in, and why it matters — the intellectual context of the work"
  topics:
    type: string
    format: prose
    description: "Per-section breakdown of the paper's key arguments and findings"

sources:
  research_paper:
    hint: "Capture paper metadata and a full narrative of the thesis, methods, and findings. Use overview for the intellectual context and topics for the section-by-section breakdown."
---
%%
field: name
description: Paper title
%%
%%
field: authors
description: Authors of the paper, comma-separated
%%
%%
field: publication_date
description: Publication date (ISO-8601)
%%
%%
field: venue
description: Journal, conference, or preprint venue
%%
%%
field: abstract_summary
description: What this paper argues or demonstrates — one paragraph
%%
%%
field: overview
description: Write 2–4 paragraphs of intellectual context. Explain what question this paper addresses, who wrote it and from what institution, what field or debate it sits in, and why it matters. Use wikilinks for named authors [[slug.person|Name]] and organizations [[slug.organization|Name]].
%%
%%
field: topics
description: Write one ### subsection per major section or argument from the TOC. For each: (1) prose explaining the argument, finding, or methodology in that section — use wikilinks for named concepts [[slug.concept|Name]] and people [[slug.person|Name]]; (2) 2–4 bullet points capturing the key claims or results. Example: "### Section Name\nThe authors argue that...\n- Key finding\n- Implication"
%%
# {{name}}

## Publication Details
- Authors: {{authors}}
- Date: {{publication_date}}
- Venue: {{venue}}

## Abstract
{{abstract_summary}}

## Overview
{{overview}}

## Topics
{{topics}}

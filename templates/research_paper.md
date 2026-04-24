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

sources:
  research_paper:
    hint: "Capture paper metadata: authors, date, venue, and a summary of the thesis or findings. The sections are context nodes."
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
# {{name}}

## Publication Details
- Authors: {{authors}}
- Date: {{publication_date}}
- Venue: {{venue}}

## Abstract Summary
{{abstract_summary}}

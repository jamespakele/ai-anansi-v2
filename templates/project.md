---
entity_type: project
atomic: true
merge_strategy: container
template_version: "2.0"
description: "A named initiative with an end date"

atomic_criteria: >
  Represents one real-world project with a defined goal and end date.
  Project-specific meeting notes and tasks belong downstream in context/task nodes.

identity_fields:
  name:
    type: string
    required: true
  goal:
    type: string
    description: "What the project aims to achieve"
  status:
    type: string
    description: "Active, completed, on-hold, cancelled"
  end_date:
    type: string
    description: "Target or actual completion date"
  summary:
    type: string
    description: "One-paragraph source-agnostic description"
  content:
    type: string
    format: prose
    description: "Source-agnostic 2–4 paragraph profile: what this project is trying to achieve, why it matters, who is driving it, and what makes it significant"

roster_sections:
  contributors:
    source_field: contributors
    render_as: "## Contributors"
    row_format: "- [[-{slug}|{name}]] — {role}"
    dedupe_by: [slug, role]

sources:
  meeting_summary:
    hint: "Check project name in agenda items, ownership attributions, project status updates"
  email_thread:
    hint: "Check project references in subject lines, action items, and project status mentions"
  research_paper:
    hint: "Check for funded projects or research initiatives"
  container:
    hint: "Check for named initiatives or projects referenced"
---
%%
field: name
description: Project name
%%
%%
field: goal
description: What the project aims to achieve, source-agnostic
%%
%%
field: status
description: Current status (Active, Completed, On-hold, Cancelled)
%%
%%
field: end_date
description: Target or actual completion date
%%
%%
field: summary
description: One-paragraph description of the project independent of any source
%%
%%
field: content
description: Write 2–4 paragraphs on this project. Cover what it is trying to achieve, why it matters, who is driving it, its current state, and what makes it significant. Do not reference this specific document — write as a durable, source-agnostic knowledge entry.
%%
# {{name}}

## Identity
- Goal: {{goal}}
- Status: {{status}}
- End Date: {{end_date}}

## Summary
{{summary}}

## Content
{{content}}

## Contributors

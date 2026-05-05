---
entity_type: project
template_class: identity
atomic: true
merge_strategy: container
template_version: "2.1"
description: "A named initiative with a specific outcome and end date — PARA Project"

atomic_criteria: >
  Represents one real-world project with a defined goal and end date.
  Per Forte (Building a Second Brain Ch. 5; PARA Method Ch. 2): a project
  has a specific completable outcome AND a finite end date AND active
  commitment. Project-specific meeting notes and tasks belong downstream
  in context/task nodes.

identity_fields:
  name:
    type: string
    required: true
  goal:
    type: string
    description: "The specific completable outcome the project is working toward — Forte's first defining trait of a Project (Building a Second Brain, Ch. 5: 'a specific, clear outcome that needs to happen in order for them to be checked off as complete, such as finalize, green-light, launch, or publish'). Stated as a verb-noun phrase that can be marked done."
  status:
    type: string
    description: "Active, completed, on-hold, cancelled. Distinct from active commitment in the source — `status` is the categorical state; commitment evidence lives in the source-side Evidence field of the extraction."
  end_date:
    type: string
    description: "Target or actual completion date — Forte's second defining trait of a Project (PARA Method, Ch. 3: 'A deadline adds a time limit to achieving your goal'). A hard date, target quarter, or other finite endpoint. Indefinite framing ('someday', 'eventually') disqualifies the project."
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
  email:
    hint: "Check project references in subject lines, action items, and project status mentions"
  email_thread:
    hint: "Check project references across the thread — subject lines, status updates, and accumulated commitments"
  newsletter:
    hint: "Check named initiatives, launches, and milestones called out as the user's work"
  meeting:
    hint: "Check project name in agenda items, ownership attributions, project status updates, and decisions"
  presentation:
    hint: "Check named initiatives or projects the slides describe the user as driving"
  speech:
    hint: "Check named initiatives the speaker presents as their own active work"
  social_media:
    hint: "Check named launches, ships, or milestones the user is attributing to themselves"
  workplace_memo:
    hint: "Check named initiatives, deliverables, and commitments with deadlines"
  company_update:
    hint: "Check named initiatives, launches, deliverables, and milestones owned by the user"
  book:
    hint: "Check for funded projects, named research initiatives, or specific deliverables"
  generic_prose:
    hint: "Check for named initiatives or projects referenced as the user's active work"
toc_structure: "none"
---
%%
field: name
description: Project name
%%
%%
field: goal
description: The specific completable outcome the project is working toward, source-agnostic. Verb-noun phrase that can be marked done. Forte's first defining trait of a Project (BASB Ch. 5).
%%
%%
field: status
description: Current status (Active, Completed, On-hold, Cancelled)
%%
%%
field: end_date
description: Target or actual completion date. Forte's second defining trait of a Project — a deadline or finite timeframe. Indefinite framing disqualifies.
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

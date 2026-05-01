---
entity_type: task
template_class: utility
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: "A single actionable item with one owner — source-specific"

identity_fields:
  name:
    type: string
    required: true
  assignee:
    type: string
    description: "Person responsible for completing this task"
  due_date:
    type: string
    description: "When this task should be completed"
  status:
    type: string
    description: "Open, in-progress, done, blocked"
  completion_criterion:
    type: string
    description: "What 'done' looks like for this task"
  context:
    type: string
    description: "Brief context: why this task exists"
  content:
    type: string
    format: prose
    description: "2–4 paragraphs explaining the full context of this task: why it exists, what problem it solves, who is accountable, and what success looks like beyond the completion criterion"

sources:
  meeting_summary:
    hint: "Capture action items assigned to specific people. Look for 'will do', 'action:', 'follow up'"
  email_thread:
    hint: "Capture explicit commitments or assigned actions from the thread"
  research_paper:
    hint: "Capture future work items or recommendations for action"
  container:
    hint: "Capture action items or tasks referenced in the document"
toc_structure: "none"
---
%%
field: name
description: Name of the task (verb + object format preferred)
%%
%%
field: assignee
description: Person responsible for this task
%%
%%
field: due_date
description: When this task should be completed
%%
%%
field: status
description: Task status (Open, In-progress, Done, Blocked)
%%
%%
field: completion_criterion
description: What done looks like for this task
%%
%%
field: context
description: Brief context explaining why this task exists
%%
%%
field: content
description: Write 2–4 paragraphs on the full context of this task. Cover why it exists, what problem it solves, who is accountable, and what meaningful success looks like. You may reference this specific document as the source.
%%
# {{name}}

## Details
- Assignee: {{assignee}}
- Due: {{due_date}}
- Status: {{status}}

## Completion Criterion
{{completion_criterion}}

## Context
{{context}}

## Content
{{content}}

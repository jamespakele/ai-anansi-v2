---
name: task-update
description: >
  Updates an existing TickTick task. Handles: change title, update content, reschedule
  due date, move to a different list, change priority. Does not handle matrix/quadrant
  changes — use task-set-matrix for that. Looks up the task by keyword search or ID
  before updating. Triggers: "update task [title]", "reschedule [task]", "move [task]
  to [list]", "change the due date on [task]", "rename [task]", "add context to [task]",
  "/task-update", "edit [task]", "update the description of [task]", "change [task]
  title to [new title]".
---

Before updating any task, read the following references from this plugin's root directory
(two levels up from this SKILL.md: `../../references/`):

- `references/adapters/ticktick.md` — MCP tool names, known list IDs, update parameters
- `references/task-manager-os.md` — context-to-list mapping, timezone

## Steps

1. Parse the update request — identify the target task and what needs to change.

2. Find the task:
   - If the user provides an ID, use `fetch` directly
   - Otherwise use `search_task` with the task title or keywords, then confirm which task
     the user means if multiple results match

3. Show what you're about to change:
   > Updating: "[current title]" — changing [field] from [old] to [new]

   Proceed immediately for unambiguous changes.

4. Call `update_task` with `task_id` and a `task` object containing only the fields
   being changed. Do not overwrite fields that are not being updated.

   Key fields:
   - `title` — updated title
   - `content` — updated body/context
   - `dueDate` — ISO 8601; always set `timeZone: "America/Honolulu"`
   - `startDate` — if changing start date
   - `projectId` — if moving to a different list (use list ID from Known Lists)
   - `priority` — if changing priority directly (not quadrant — use task-set-matrix for that)

5. For matrix/quadrant changes, redirect to `task-set-matrix`. For completing a task,
   use `complete_task` directly (requires `project_id` and `task_id`).

6. Confirm the update with the changed field(s).

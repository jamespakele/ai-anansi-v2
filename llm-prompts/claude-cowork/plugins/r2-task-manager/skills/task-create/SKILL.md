---
name: task-create
description: >
  Creates a new task in TickTick using Pakele-OS routing. Infers context (which list)
  and quadrant (arena/wuwei/zheng/radar) from the task description — user can correct.
  Optionally adds the waiting overlay tag. Never creates new tags. Uses existing lists
  only; surfaces missing lists before acting. Triggers: "create a task", "add a task",
  "new task", "add to my list", "log this task", "task: [description]", "/task-create",
  "create task in [context]", "add arena task", "add wuwei task", "create a waiting task",
  or any request to capture an action item into TickTick.
---

Before creating any task, read the following references from this plugin's root directory
(two levels up from this SKILL.md: `../../references/`):

- `references/adapters/ticktick.md` — MCP tool names, known list IDs, tag list, priority values
- `references/task-manager-os.md` — context inference, quadrant definitions, matrix co-existence rule

## Steps

1. Parse the request — extract title, any explicit context, quadrant, waiting flag, due date,
   and list name if mentioned.

2. Infer missing fields:
   - **Context** → select the best matching list from the Known Lists table in `ticktick.md`
   - **Quadrant** → one of: `arena`, `wuwei`, `zheng`, `radar`
   - **Waiting** → add `waiting` tag if something is blocking the task
   - **Priority** → map from quadrant per the Priority Values table

3. If the inferred list does not exist in Known Lists, stop and tell the user. Do not silently
   fall back to Inbox or another list. Surface it as a `project-create` or `area-create` action.

4. Show a one-line summary before creating:
   > Creating: "[title]" → [list name] · [quadrant][, waiting] · [due date if set]

   Proceed immediately unless something is ambiguous enough to block.

5. Call `create_task` with:
   - `task.title` — concise title (longer context goes in `content`)
   - `task.projectId` — list ID from Known Lists
   - `task.tags` — array with the quadrant tag; include `waiting` if applicable
   - `task.priority` — mapped from quadrant
   - `task.timeZone` — always `"America/Honolulu"`
   - `task.dueDate` — ISO 8601 if a date was given
   - `task.content` — any additional context, why it matters, completion criteria

6. Confirm with the task title and list name. If the user needs to correct context or quadrant,
   handle it with `task-update` or delete and recreate.

---
name: task-set-matrix
description: >
  Sets or changes the matrix label (quadrant or waiting overlay) on a TickTick task.
  Enforces the co-existence rule: setting a quadrant (arena/wuwei/zheng/radar) removes
  all other quadrant tags but preserves the waiting overlay; setting or removing waiting
  never touches the quadrant. Triggers: "set [task] to arena", "move [task] to wuwei",
  "mark [task] as waiting", "remove waiting from [task]", "change quadrant of [task]",
  "/task-set-matrix", "that task is blocked", "unblock [task]", "escalate [task] to
  arena", "demote [task] to radar", "put [task] in zheng".
---

Before acting, read the following references from this plugin's root directory
(two levels up from this SKILL.md: `../../references/`):

- `references/adapters/ticktick.md` — MCP tool names, tag list, priority values
- `references/task-manager-os.md` — matrix co-existence rule, quadrant definitions

## Matrix Co-existence Rule (enforce this every time)

Valid tags: `arena`, `wuwei`, `zheng`, `radar` (quadrants — mutually exclusive) + `waiting` (overlay).

- **Setting a quadrant** (`arena`, `wuwei`, `zheng`, `radar`):
  Remove all other quadrant tags. Keep `waiting` if it is already present.

- **Adding `waiting`**:
  Add it to the existing tags. Do not touch the quadrant tag.

- **Removing `waiting`**:
  Remove only `waiting`. Leave the quadrant tag unchanged.

- **Never** leave a task with two quadrant tags (e.g. `arena` + `radar`).

## Steps

1. Parse the request — identify the target task and the desired matrix change.

2. Find the task:
   - If ID is known, use `fetch` directly
   - Otherwise use `search_task`, confirm if multiple results

3. Read the task's current `tags` array from the fetched task.

4. Compute the new tags array:
   - Identify which of the five tags are currently present
   - Apply the co-existence rule to produce the correct new set

5. Show what you're about to change:
   > Setting matrix: "[task title]" → [old tags] → [new tags]

   Proceed immediately.

6. Call `update_task` with:
   - `task_id`
   - `task.tags` — the new computed tags array
   - `task.priority` — if the quadrant changed, update priority to match (see Priority Values in adapter)

7. Confirm with the new matrix state.

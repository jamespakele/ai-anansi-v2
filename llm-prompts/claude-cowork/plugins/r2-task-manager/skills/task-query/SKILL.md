---
name: task-query
description: >
  Filters and searches tasks in TickTick using Pakele-OS context and quadrant filters.
  Routes to filter_tasks (structured query) or search_task (keyword search) based on
  the request. Returns tasks with title, list, quadrant tag, waiting status, priority,
  and due date. Triggers: "what tasks do I have in [context]", "show me my arena tasks",
  "what's in [list]", "find tasks about [topic]", "what's due this week", "show wuwei
  tasks", "list my dcs tasks", "search tasks for [keyword]", "/task-query", "what do I
  have on my plate", "filter tasks by [quadrant/context/date]", "show waiting tasks",
  "what's blocked".
---

Before querying, read the following references from this plugin's root directory
(two levels up from this SKILL.md: `../../references/`):

- `references/adapters/ticktick.md` — MCP tool names, known list IDs, tag and filter options
- `references/task-manager-os.md` — context-to-list mapping, quadrant definitions

## Routing Logic

Use **`filter_tasks`** for structured queries:
- Filter by context (list): resolve context name → list IDs from Known Lists table
- Filter by quadrant: pass as `tag` array (e.g. `["arena"]`)
- Filter by waiting: include `"waiting"` in the `tag` array
- Filter by date range: `startDate` / `endDate` in ISO 8601
- Filter by priority: map from quadrant if needed
- Multiple filters are AND-combined

Use **`search_task`** for keyword/topic searches when no structured filter applies.

Combine both when useful: search for keyword, then filter results by context or quadrant.

## Output Format

For each task returned, show:
- Title
- List name (context)
- Tags (quadrant + waiting if present)
- Due date (if set)
- Priority indicator

Group results by context or quadrant if the query spans multiple. Keep the output scannable —
one task per line with key fields inline. Do not dump full `content` fields unless the user asks.

## Edge Cases

- If no list ID is known for a named context, list what is known and ask for clarification.
- If a search returns zero results, say so clearly and suggest broadening the query.
- Cap display at 20 tasks; offer to refine if more are returned.

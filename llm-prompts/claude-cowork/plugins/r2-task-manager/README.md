# r2-task-manager

Task management interface layer for Pakele-OS. Provides adapter-backed skills for
creating, querying, updating, and routing tasks, projects, and areas in TickTick —
with ClickUp support planned.

## Skills

| Skill | What it does |
|-------|-------------|
| `task-create` | Create a task — infers context (list) and quadrant from description |
| `task-query` | Filter or search tasks by context, quadrant, date, or keyword |
| `task-update` | Update task fields — title, content, due date, list |
| `task-set-matrix` | Set or change a task's quadrant or waiting overlay |
| `project-create` | Create a new list in the 1. Projects folder |
| `area-create` | Create a new list in the 2. Areas folder |

## Architecture: Adapter Pattern

Skills are tool-agnostic. They speak in Pakele-OS terms (contexts, quadrants, list names).
The active adapter file (`references/adapters/ticktick.md`) maps those terms to specific
MCP tool calls and project IDs.

To add a new task manager (e.g. ClickUp):

1. Fill in `references/adapters/clickup.md` with the ClickUp MCP tool mappings
2. Document the workspace hierarchy (spaces → folders → lists) and list IDs
3. Map the four matrix tags to ClickUp's label/status model
4. Update each SKILL.md to detect which adapter is active based on connected MCP tools

## Matrix Labels

Five tags total. Never create new tags.

**Quadrant (mutually exclusive — Eisenhower via Zero-Null):**
- `arena` — important + due ≤ 48h or overdue → do now
- `wuwei` — important + not urgent → deep work, plan and invest
- `zheng` — not important + urgent → batch, delegate, automate
- `radar` — not important + not urgent → monitor only, weekly review

**Overlay (pairs with any quadrant):**
- `waiting` — something external is blocking this task

## Known Limitation

TickTick's MCP `create_project` tool does not support folder group assignment. After
creating a project or area list, it must be manually dragged into the correct folder
(1. Projects or 2. Areas) in the TickTick app.

## References

- `references/task-manager-os.md` — context definitions, quadrant rules, inference logic
- `references/adapters/ticktick.md` — active adapter: MCP tools, list IDs, naming convention
- `references/adapters/clickup.md` — planned adapter (stub)

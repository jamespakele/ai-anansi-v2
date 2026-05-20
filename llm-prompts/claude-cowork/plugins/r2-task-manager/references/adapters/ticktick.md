# TickTick Adapter

Active adapter for r2-task-manager. Maps skill operations to TickTick MCP tools.

## MCP Tool Reference

| Operation | MCP Tool |
|-----------|----------|
| Create task | `create_task` |
| Update task | `update_task` (requires `task_id` + `task` fields to update) |
| Complete task | `complete_task` (requires `project_id` + `task_id`) |
| Fetch task by ID | `fetch` (requires `id`) |
| Search tasks (keyword) | `search_task` (requires `query`) |
| Filter tasks (structured) | `filter_tasks` (accepts `tag`, `projectIds`, `startDate`, `endDate`, `priority`, `status`) |
| List all projects/lists | `list_projects` |
| Create project/list | `create_project` (requires `name`; accepts `color`, `kind`, `view_mode`) |
| Update project/list | `update_project` (requires `project_id`) |

## Folder Groups

| Folder | Group ID | Purpose |
|--------|----------|---------|
| 1. Projects | `69fb6fdcebdef90003000116` | Active projects with outcomes / deadlines |
| 2. Areas | `69fb6fcdebdef9000300010a` | Ongoing responsibilities |

⚠️ **Limitation**: `create_project` does not support assigning a folder group. Newly created
lists land ungrouped and must be manually moved to the correct folder in the TickTick app.
Always tell the user this after creating a project or area.

## Known Lists

| List ID | Name | Context | Folder |
|---------|------|---------|--------|
| `69fb6fc9ebdef90003000103` | 🤖ai-anansi-v2 | ai | Projects |
| `69fb7090ebdef900030001c1` | ☕anykine-concon | anykine | Projects |
| `69fb735aebdef900030003a9` | 🤖ai-paaluhi-automation | ai | Projects |
| `69fb6f74ebdef900030000eb` | 💰home-finances | me | Areas |
| `69fb703aebdef90003000157` | ♻home-routine | me | Areas |
| `69fb71e9ebdef9000300026d` | 🚈home-commute | me | Areas |
| `69fb7238ebdef9000300029d` | ♻iq-routine | iq | Areas |
| `69fb7297ebdef900030002fd` | ♻dcs-routine | dcs | Areas |
| `69fb7372ebdef900030003bf` | 🌐ai-isoc-hawaii | ai | Areas |
| `inbox` | Inbox | — | (no folder) |

## List Naming Convention

Format: `{emoji}{context}-{name}` — no space between emoji and context prefix.

Emoji guidelines (patterns, not strict rules):
- 🤖 tech / automation / AI tools
- ☕ civic / community / gatherings
- 💰 finances
- ♻ recurring responsibilities / routines
- 🚈 transit / logistics
- 🌐 web / organizations / public-facing
- 🏠 home / personal
- 📋 admin / ops
- 🔬 research / analysis

## Matrix Tags

Only these five tags exist. **Never create new tags.**

**Quadrant (mutually exclusive):** `arena` | `wuwei` | `zheng` | `radar`

**Overlay (can pair with any quadrant):** `waiting`

See `task-manager-os.md` for the co-existence rule and full definitions.

## Priority Values

| Value | Quadrant |
|-------|----------|
| 5 | arena |
| 3 | wuwei |
| 1 | zheng |
| 0 | radar |

## Task Status Values

| Value | Meaning |
|-------|---------|
| 0 | Not completed |
| 2 | Completed |
| -1 | Abandoned |

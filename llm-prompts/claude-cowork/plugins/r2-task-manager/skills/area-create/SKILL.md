---
name: area-create
description: >
  Creates a new TickTick list in the 2. Areas folder. Infers the context prefix
  (me/dcs/iq/ai/anykine) and an appropriate emoji from the area description, then
  proposes the full list name ({emoji}{context}-{name}) for confirmation before creating.
  Note: TickTick MCP does not support folder assignment — the new list must be manually
  moved to the 2. Areas folder in the TickTick app after creation. Triggers: "create
  an area", "new area", "add an area", "create [context] area", "/area-create", "I need
  a new ongoing list for [topic]", "set up an area for [responsibility]", "add a
  standing list for [topic]".
---

Before creating an area, read the following references from this plugin's root directory
(two levels up from this SKILL.md: `../../references/`):

- `references/adapters/ticktick.md` — naming convention, emoji guidelines, `create_project` params
- `references/task-manager-os.md` — context prefixes

## Projects vs Areas

- **Project** (1. Projects folder): has a defined outcome and an end state — it completes.
- **Area** (2. Areas folder): an ongoing responsibility with no end date — it continues.

If the user's request sounds like a project (time-bounded, outcome-driven), redirect to
`project-create` instead.

## Steps

1. Parse the request — extract the area topic and any explicit context or emoji.

2. Infer the context prefix from the subject matter using the rules in `task-manager-os.md`.

3. Propose a list name following the convention: `{emoji}{context}-{name}`
   - Consult the emoji guidelines in `ticktick.md`
   - Keep `{name}` short, lowercase, hyphenated (e.g. `dcs-board-ops`, `home-health`)

4. Show the proposed name and ask for confirmation or corrections before creating:
   > Proposed list name: `📋dcs-board-ops` — does this look right?

5. On confirmation, call `create_project` with:
   - `name` — the confirmed list name
   - `view_mode` — `"list"` (default)
   - `kind` — `"TASK"` (default)

6. After creation, always tell the user:
   > ⚠️ The new list was created but is not yet in the 2. Areas folder. Open TickTick
   > and drag it into the correct folder group manually.

7. Add the new list to the Known Lists table in `references/adapters/ticktick.md` by
   recording the returned project ID, name, context, and folder (Areas).

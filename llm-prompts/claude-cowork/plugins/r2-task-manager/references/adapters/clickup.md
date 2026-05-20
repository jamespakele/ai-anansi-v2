# ClickUp Adapter (Stub)

Planned adapter for r2-task-manager. Not yet implemented.

## Status

ClickUp MCP is available but mapping to Pakele-OS context/quadrant routing has not been
completed. When ClickUp becomes the active adapter, this file should be updated to match
the structure of `ticktick.md` — covering:

- MCP tool reference (create, update, complete, filter, search, list spaces/folders/lists)
- Folder/space structure and IDs
- Known lists with context mapping
- List naming convention (should match TickTick convention for consistency)
- Tag/label mapping to the four matrix labels (arena, wuwei, zheng, radar)
- Priority value mapping

## How to Add This Adapter

1. Connect the ClickUp MCP in Cowork and load the available tools
2. Map each operation in the "MCP Tool Reference" table above to the equivalent ClickUp tool
3. Enumerate the workspace hierarchy (spaces → folders → lists) and capture IDs for known lists
4. Document any ClickUp-specific constraints (e.g. label vs tag model, status fields)
5. Update the skill SKILL.md files to detect and use this adapter when ClickUp tools are connected

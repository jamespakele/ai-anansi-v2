---
name: anansi-delete
description: >
  Removes or archives a single Anansi note by ID. Two modes: (1) hard delete —
  permanently removes the note via anansi_delete_note, irreversible; (2) archive
  — soft-removes via anansi_archive_note, reversible; pass restore: true to
  unarchive. Always looks up the record first to confirm identity and surface the
  note ID before acting. Use when James says "delete [entity]", "remove this
  note", "archive [name]", "unarchive [name]", "restore [entity]",
  "/anansi-delete", "delete this from anansi", "soft delete [name]",
  "archive this project", "remove [person/org/event] from the vault",
  or "hide [name] from the vault".
argument-hint: "[entity name, match_key, or note ID] [delete | archive | unarchive]"
---

# anansi-delete

Remove or archive a single Anansi note. Two distinct paths — pick the right
one based on the user's intent.

| Mode | Tool | Reversible? |
|---|---|---|
| **Hard delete** | `anansi_delete_note` | ❌ Permanent |
| **Archive** | `anansi_archive_note` | ✅ Restore with `restore: true` |
| **Unarchive** | `anansi_archive_note` (restore: true) | — |

When in doubt about intent, ask. Hard delete is permanent — do not default to
it.

---

## Mode detection

**Hard delete** when the user says:
- "delete", "remove", "permanently delete", "wipe", "get rid of"

**Archive** when the user says:
- "archive", "hide", "soft delete", "deactivate", "put away", "shelve"

**Unarchive** when the user says:
- "unarchive", "restore", "bring back", "reactivate", "un-hide"

When ambiguous (e.g. "remove this note"), **default to archive** and confirm:

> "I'll archive this rather than permanently delete it — is that right? Or do
> you want a hard delete?"

---

## Step 1 — Find the record and get the note ID

The MCP tools operate on the note's **internal UUID** (`id`), not the
match_key. You must retrieve the record first.

**Try `anansi_get` first** with the inferred match_key:

```json
{ "match_key": "person:john-doe" }
```

If `anansi_get` returns nothing, fall through to `anansi_search`:

```json
{ "q": "John Doe" }
```

From the result, extract:
- `id` — the note UUID (required for both tools)
- `name` — for display in confirmation
- `entity_type` — for display in confirmation
- `match_key` — for display in confirmation

**If no match found:** Tell the user clearly. Do not proceed.

> "No record found for 'John Doe' in the vault. Nothing was deleted or
> archived."

**If multiple matches surface:** List all candidates. Ask the user to
identify the specific one before continuing. Never delete ambiguously.

---

## Step 2 — Confirm before acting

Always confirm with the user before executing either operation. Show what
you're about to do and which note it targets.

**Hard delete confirmation:**

```
⚠ *Permanent delete* — this cannot be undone.

• *name:* John Doe
• *type:* person
• *match_key:* person:john-doe
• *id:* {uuid}

Type "yes" to permanently delete this note from Anansi.
```

**Archive confirmation:**

```
Archive this note? It will be hidden from search and queries but can be
restored.

• *name:* John Doe
• *type:* person
• *match_key:* person:john-doe
• *id:* {uuid}

Confirm? (yes / no)
```

**Unarchive confirmation:**

```
Restore this note to the active vault?

• *name:* John Doe
• *type:* person
• *match_key:* person:john-doe
• *id:* {uuid}

Confirm? (yes / no)
```

Only proceed after explicit confirmation. If the user says no or hesitates,
stop and ask what they'd prefer.

---

## Step 3 — Call the tool

### Hard delete

```json
{
  "id": "{note_uuid}"
}
```

Call: `anansi_delete_note`

### Archive

```json
{
  "id": "{note_uuid}"
}
```

Call: `anansi_archive_note`

### Unarchive / Restore

```json
{
  "id": "{note_uuid}",
  "restore": true
}
```

Call: `anansi_archive_note`

---

## Step 4 — Report

**Hard delete:**
```
*Anansi* — deleted ✓
• {name} [{entity_type}]
• match_key: {match_key}
• Permanently removed. This cannot be undone.
```

**Archive:**
```
*Anansi* — archived ✓
• {name} [{entity_type}]
• match_key: {match_key}
• Hidden from vault. Restore anytime with: "unarchive {name}"
```

**Unarchive:**
```
*Anansi* — restored ✓
• {name} [{entity_type}]
• match_key: {match_key}
• Back in the active vault.
```

---

## Error handling

| Situation | Action |
|---|---|
| `anansi_delete_note` not available | Tell the user. Do not attempt workarounds. |
| `anansi_archive_note` not available | Tell the user. |
| Tool returns error | Show the error. Do not retry silently. |
| User confirms hard delete then changes mind | Cannot undo if tool already called. Be clear about this upfront. |
| Note ID not present in anansi_get result | Ask the user to provide the UUID directly, or check Datasette. |

---

## Hard Rules

- Never hard-delete without explicit user confirmation in the chat.
- Never hard-delete multiple notes in a single invocation — one note, one
  confirmation, one call.
- When ambiguous between hard delete and archive, **default to archive**.
- Never call `anansi_purge` from this skill — that's a source-level bulk
  operation handled by `anansi:anansi-purge`.

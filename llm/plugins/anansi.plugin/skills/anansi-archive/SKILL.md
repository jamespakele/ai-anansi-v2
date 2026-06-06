---
name: anansi-archive
description: >
  Soft-archive a single Anansi note by ID or match_key. Routes through the
  skill layer so source: "skill" is set, permitting execution in scheduled
  context. Resolves match_key → UUID via anansi_get when only match_key is
  provided. Optionally appends an Explainability Contract line before
  archiving when a reason is given. Use when the user says "archive this
  note", "soft-archive [name]", "/anansi-archive", "shelve [name]", or
  when invoked programmatically by skills that need to archive notes
  (e.g. pm-review merge flow).
argument-hint: "[note_id or match_key] [optional reason]"
---

# anansi-archive

Soft-archive a single Anansi note. Non-destructive — edges, embeddings, and
content are preserved. The note's `entity_type` gets prefixed with `archive-`
(e.g. `discussion` → `archive-discussion`), hiding it from normal searches.

This skill exists to route archive operations through the skill layer
(`source: "skill"`), which is required for writes in scheduled execution
contexts.

> **Not `anansi-delete`.** This skill is archive-only. It does not offer
> hard-delete or unarchive. For hard-delete, use `anansi:anansi-delete`.
> For restore/unarchive, use `anansi:anansi-delete` with the unarchive mode.

---

## Inputs

| Field | Required | Notes |
|---|---|---|
| `note_id` | One of these two | Anansi note UUID |
| `match_key` | One of these two | e.g. `project:konohiki-plugin` |
| `reason` | No | Short human-readable reason — appended to the note before archiving |

Resolution order: if `note_id` is provided, use it directly. If only
`match_key` is provided, resolve to UUID via `anansi_get(match_key=...)` first.

---

## Step 1 — Resolve the note ID

**If `note_id` was provided:** use it directly. Skip to Step 2.

**If only `match_key` was provided:** call `anansi_get`:

```json
{ "match_key": "project:konohiki-plugin" }
```

Extract the `id` field from the response. Also capture `name`, `entity_type`,
and `match_key` for the confirmation report.

**If no match found:** report and stop.

> "No note found for match_key '{match_key}'. Nothing was archived."

**If `note_id` was provided but you need the note's name for reporting:**
optionally call `anansi_get` with `{ "id": "{note_id}" }` to fetch display
fields. This is optional — if you already have the name from context, skip it.

---

## Step 2 — Append reason (if provided)

If the caller provided a `reason`, append an Explainability Contract line
to the note's content before archiving. This satisfies the Explainability
Contract (os.md §9).

Call `anansi_update_note`:

```json
{
  "note_id": "{resolved_id}",
  "content": "{existing_content}\n\n⚙️ AI [{ISO timestamp}]: archived — {reason}",
  "source": "skill"
}
```

If the note has no existing content, set content to just the explainability
line:

```json
{
  "note_id": "{resolved_id}",
  "content": "⚙️ AI [{ISO timestamp}]: archived — {reason}",
  "source": "skill"
}
```

If `reason` is not provided, skip this step entirely.

---

## Step 3 — Archive the note

Call `anansi_archive_note`:

```json
{
  "note_id": "{resolved_id}",
  "source": "skill"
}
```

---

## Step 4 — Report

On success:

```
*Anansi* — archived ✓
• {name} [{entity_type}]
• match_key: {match_key}
• Hidden from vault. Restore with: anansi-delete unarchive {name}
```

If a reason was appended:

```
*Anansi* — archived ✓
• {name} [{entity_type}]
• match_key: {match_key}
• Reason logged: {reason}
• Hidden from vault. Restore with: anansi-delete unarchive {name}
```

---

## Error handling

| Situation | Action |
|---|---|
| Note not found (by ID or match_key) | Report clearly and stop. Do not error silently. |
| Note already archived (server returns error) | Report: "Note is already archived." Treat as success — idempotent, not a failure. |
| `anansi_archive_note` not available | Tell the caller. Do not attempt workarounds. |
| `anansi_update_note` fails (reason append) | Warn but continue with the archive. The reason is nice-to-have, not blocking. |
| `anansi_get` returns multiple matches | List candidates and ask the caller to confirm. Do not archive ambiguously. |

---

## Constraints

- **Archive only, never delete.** Callers that need hard-delete use
  `anansi:anansi-delete`.
- **Always append an Explainability Contract line** before archiving if a
  reason is given.
- **Resolving match_key → UUID** requires one `anansi_get` read call before
  the archive write — this is acceptable overhead.
- **No user confirmation required.** Unlike `anansi-delete`, this skill does
  not prompt for confirmation. Archiving is reversible and safe for
  programmatic callers.
- **Always include `"source": "skill"`** in every write call payload.

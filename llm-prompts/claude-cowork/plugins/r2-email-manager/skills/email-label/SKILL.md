---
name: email-label
description: >
  Applies Gmail labels to threads after triage, based on Pakele-OS context
  assignments. Creates the email-manager label hierarchy if not present.
  Marks processed threads with email-manager/processed and optionally archives
  them from the inbox. Takes a triage output file or a list of thread IDs with
  context assignments as input. Triggers: "label these emails", "apply labels",
  "archive processed email", "mark email as processed", "email-label",
  "/email-label", "label after triage", "apply triage labels",
  "label and archive", "clean up inbox after triage".
argument-hint: "[triage output file | thread IDs] [--archive] [--no-archive]"
---

# email-label

Applies Gmail labels to threads based on triage output. The labeling layer
that closes the loop between `inbox-pull` → `email-triage` → Gmail inbox state.

---

## Flags

| Flag | Meaning |
|------|---------|
| *(default)* | Apply context labels + mark processed. Leave in inbox. |
| `--archive` | Apply labels then remove `INBOX` label (archives the thread). |
| `--no-archive` | Explicitly keep in inbox even if triage suggested archiving. |

---

## Step 1 — Read `../../references/gmail-adapter.md`

Load the label names, label hierarchy, and context-to-label mapping before
any API calls.

---

## Step 2 — Identify input

Determine what threads to label:

- **Triage output file path provided**: read the file. Extract all thread IDs
  and their assigned contexts and quadrants from the triage output.
- **Thread IDs provided directly** (with context assignments): use as-is.
- **No explicit input**: check if a triage output file was produced earlier
  in the conversation. If found, confirm with the user before using it.

The minimum needed per thread: `thread_id` + `context` (one of me/dcs/iq/ai/anykine).
Quadrant is used for the label only if a quadrant sub-label already exists —
do not create quadrant sub-labels unless explicitly requested.

---

## Step 3 — Ensure label hierarchy exists

Call `list_labels` once to get all current Gmail labels.

Check for the presence of each required label:

```
personal
dcs
iq
ai
anykine
calendar
email-manager/processed
email-manager/needs-reply
```

For any label that does not exist: call `create_label` to create it.
Create the parent `email-manager` label first if the `email-manager/processed`
or `email-manager/needs-reply` labels are absent.

Report which labels were created (skip report if all already existed).

---

## Step 4 — Apply labels

For each thread:

1. Determine the target label from the context assignment:
   - `me` → `personal`
   - `dcs` → `dcs`
   - `iq` → `iq`
   - `ai` → `ai`
   - `anykine` → `anykine`
   - `calendar` → `calendar` (calendar invites and scheduling threads)

2. Always add `email-manager/processed` to every thread being labeled.

3. If the triage output flagged a thread as needing a reply, also add
   `email-manager/needs-reply`.

4. If `--archive` flag is active: remove the `INBOX` label from the thread
   (i.e., archive it). Confirm the archive count before proceeding if more
   than 10 threads will be archived.

Use the Gmail MCP to apply label changes. The exact tool call depends on
what the Gmail MCP exposes — use `modify_thread` or equivalent to add/remove
labels by ID.

**Note on label IDs**: Gmail requires label IDs, not names, for modification
calls. Map names → IDs using the `list_labels` response from Step 3.

---

## Step 5 — Report

After all threads are labeled:

```
Labels applied — {YYYY-MM-DD}

Threads labeled: {N}
  personal:   {N}
  dcs:        {N}
  iq:         {N}
  ai:         {N}
  anykine:    {N}
  calendar:   {N}

Marked processed:    {N}
Marked needs-reply:  {N}
Archived:            {N}  (only if --archive was active)

Labels created this run: {list, or "none"}
```

---

## Design notes

**Processed is permanent signal.** The `email-manager/processed` label is
what `inbox-pull` uses to exclude already-seen threads from future pulls. Never
remove it without an explicit user request.

**Archive is irreversible in feel.** Warn the user before archiving more than
10 threads in a single run. A mistaken bulk archive is annoying to undo.

**Label creation is idempotent.** Always check before creating — duplicate
labels clutter Gmail and confuse future queries.

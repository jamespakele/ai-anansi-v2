---
name: triage-ingest
description: >
  Receives a list of pre-classified action items from inbox-triage (context and
  quadrant already assigned) and creates or updates tasks in TickTick via the
  adapter layer. Handles deduplication — high-confidence title match appends a
  timestamped update note instead of creating a duplicate. Never re-classifies
  context or quadrant; that work belongs to the calling skill. Returns a summary
  of tasks created, updated, and context breakdown. Intended to be called
  programmatically by email-manager:inbox-triage, not invoked directly by the
  user. Triggers: called by inbox-triage, "triage-ingest", "/triage-ingest",
  "push classified items to ticktick", "create tasks from triage output".
argument-hint: "[classified items payload from inbox-triage]"
---

# triage-ingest

Adapter-backed task ingestion. Receives classified action items, writes them to
the task manager. Does not classify — context and quadrant arrive pre-assigned
from the calling skill.

---

## Inputs

A list of classified action items. Each item has:

| Field | Required | Description |
|-------|----------|-------------|
| `title` | yes | Action text, verb + object, ≤60 chars |
| `context` | yes | `dcs` / `iq` / `ai` / `me` / `anykine` |
| `quadrant` | yes | `arena` / `wuwei` / `zheng` / `radar` |
| `due_date` | no | ISO 8601 date, omit if absent |
| `lede` | no | Source block lede sentence |
| `why` | no | Source block why sentence |
| `narrative` | no | 2–3 sentence Smart Brevity context digest |
| `completion_criterion` | no | What done looks like |
| `whisper` | no | Attached Varys whisper lede + why |
| `source_slug` | no | Atomized file slug for provenance |

---

## Step 1 — Load adapter reference

Read `../../references/adapters/ticktick.md` for MCP tool names, known list IDs,
tag names, and priority values. All task operations use this adapter.

---

## Step 2 — Resolve lists

For each unique context in the incoming items, confirm the target TickTick list
exists in the Known Lists table.

If a required list is missing: surface it to the user and stop. Do not fall
back silently to Inbox. Use `r2-task-manager:project-create` or
`r2-task-manager:area-create` to create the missing list, then retry.

---

## Step 3 — Deduplicate

For each item, search TickTick using the first 6–8 significant words of the
title via `search_task`.

- **High-confidence match** (same title shape, same context project list):
  **update** — append a timestamped note block to the existing task's content
  field. Do not change title, quadrant, or due date on the existing task.
- **Low-confidence or no match**: **create** new task.

Prefer creating a new task over a wrong merge. Accumulated context in a task
description is valuable — a mismatched update is worse than a brief duplicate.

---

## Step 4 — Create new tasks

For each item with no dedup match, call `create_task` with:

| TickTick field | Value |
|----------------|-------|
| `title` | Item title |
| `projectId` | List ID for item's context (from Known Lists) |
| `priority` | Per quadrant: arena=5, wuwei=3, zheng=1, radar=0 |
| `dueDate` | ISO 8601 if provided, omit if absent |
| `timeZone` | `America/Honolulu` always |
| `tags` | `["{quadrant}"]` — one quadrant tag only, never create new tags |
| `content` | Task description (see format below) |

### Task description format — new task

```
{lede — one sentence, the core action or outcome.}

**Why it matters:** {why sentence.}

**Context:** {narrative — 2–3 sentence Smart Brevity digest.}

**Completion criterion:** {what done looks like, if provided.}

[If whisper present:]
📡 Signal: {whisper lede — ≤15 words.}
↳ {whisper why — ≤20 words.}

[If source_slug present:]
Source: {source_slug}
```

Omit any section whose field was not provided. Do not invent content for
missing fields.

---

## Step 5 — Update existing tasks

For each item matched to an existing task, call `update_task` appending to
the task's `content` field:

```
---
[{YYYY-MM-DD} update from {source_slug}]
{New context — 1–3 sentences. What changed, what's new.}
[📡 Signal: {whisper lede} — if a new whisper applies to this update.]
```

Do not modify `title`, `tags`, `priority`, or `dueDate` on existing tasks
during a triage-ingest run. Those fields belong to the user to manage.

---

## Step 6 — Return summary

Return a summary to the calling skill (`inbox-triage`) for display:

```
Triage ingest complete — {source_slug} — {YYYY-MM-DD}

Tasks created:  {N}  (arena: {N} · wuwei: {N} · zheng: {N} · radar: {N})
Tasks updated:  {N}
Whispers attached: {N}

Context breakdown:
  dcs: {N} · iq: {N} · ai: {N} · me: {N} · anykine: {N}
```

---
name: task-report
description: >
  Generates James's daily task report from live TickTick state. ONE Things
  checked first: if a ONE Thing task with today's due date exists per context
  (dcs/iq/ai/me), it is read as-is; if not, synthesized from open tasks and
  created in TickTick with today's due date - never re-generated if already set
  today. Arena and Wu Wei render in Smart Brevity format (lede + why + go
  deeper). Zheng renders as lede only with waiting-on and due date. Radar is a
  flat list. Varys whispers attached to tasks appear inline. anykine does not
  get a ONE Thing. Triggers: "run task report", "generate my daily report",
  "/task-report", "what's on my plate today", "daily report", "task summary",
  "show my tasks", "morning report", "end of day report".
argument-hint: "[optional: context filter - dcs|iq|ai|me|anykine|all (default: all)]"
---

# task-report

Generates James's daily task report from live TickTick state. Not a snapshot of what came in today — a report of everything currently on the board, organized by quadrant and context, anchored by a stable ONE Thing per context.

---

## When to invoke

Invoke when:
- James wants to see his current task state across all contexts
- The user says "run task report", "/task-report", "what's on my plate today", "daily report", "task summary", "morning report", "show my tasks"
- After an `inbox-triage` run, to see the updated board
- Morning planning or end-of-day review

Optional argument: a context filter (`dcs`, `iq`, `ai`, `me`, `anykine`) to scope the report to one context. Default is all contexts.

---

## Step 1 — ONE Things check (idempotent)

For each context — `dcs`, `iq`, `ai`, `me` — check TickTick for an existing ONE Thing:

**Query:** task in project `ONE Thing`, tagged with context label, due date = today.

**If found:** read the task title and description. Display as-is. If marked complete, show with ✓. Do not replace it.

**If not found:** synthesize a ONE Thing for this context from current open tasks in that context's project/list.

Apply the ONE Thing test:
> "What is the ONE thing I can do right now such that by doing it, everything else becomes easier or unnecessary?"

This is not the most urgent task — it is the most leveraged. Look for blockers that unfreeze multiple downstream items, multipliers that accelerate the whole context, dependencies where James is the single point of action. Prioritize `arena` tasks as ONE Thing candidates; a well-placed `wuwei` task can outrank an `arena` task if it removes a structural blocker.

Create a task in TickTick:
- Project: `ONE Thing`
- Title: verb + object format, ≤60 chars
- Tag: context label (e.g., `dcs`)
- Due date: today
- Content: one sentence rationale — what this unblocks or multiplies, ≤20 words

Also synthesize an **OVERALL** ONE Thing — the action that creates lift across two or more contexts. Apply the same test across the full open board. Create in TickTick (project: `ONE Thing`, tag: `overall`, due: today) if not already set for today.

**`anykine` does not get a ONE Thing.** Catch-all contexts lack the singularity the ONE Thing requires.

---

## Step 2 — Pull task state from TickTick

Fetch all open tasks, excluding completed and cancelled. Group by:
1. Quadrant (priority field: 5=arena, 3=wuwei, 1=zheng, 0=radar)
2. Context (project/list: dcs / iq / ai / me / anykine)

Within each quadrant, order by: overdue → due today → due within 48h → undated (newest first).

---

## Step 3 — Render the report

```
# Task Report — {YYYY-MM-DD}

---

## 🎯 ONE Things

**OVERALL:** {one_thing}
↳ {rationale — what this unblocks across contexts}

**dcs:** {one_thing}  [✓ if complete]
↳ {rationale}

**iq:** {one_thing}
↳ {rationale}

**ai:** {one_thing}
↳ {rationale}

**me:** {one_thing}
↳ {rationale}

---

## 1. Arena
*Urgent + important. Do now.*

### dcs
**{Task title}**
{Lede — one sentence, the core action or outcome, ≤25 words.}

**Why it matters:** {One sentence on stakes or deadline.}

{Go deeper — 2–3 sentences. Who's involved, what's blocking, what done looks like.}

Due: {date}
📡 {Whisper lede}
↳ {Whisper why}

---

## 2. Wu Wei
*Important. Not urgent. Do this week.*

### dcs
**{Task title}**
{Lede}

**Why it matters:** {Why}

{Go deeper}

[📡 whisper if attached]

---

## 3. Zheng
*Scheduled, delegated, or waiting.*

### dcs
**{Task title}** — {lede sentence only}
Waiting on: {person} / Due: {date}

---

## 4. Radar
*Monitor. No action needed now.*

- dcs: {task title}
- iq: {task title}
- ai: {task title}
- me: {task title}
- anykine: {task title}
```

### Rendering rules

**Arena and Wu Wei — full SB format:**
- Lede: one sentence ≤25 words, the core action or outcome. Standalone — no "as per…" references.
- Why it matters: one sentence adding stakes or downstream consequence. Does not repeat the lede.
- Go deeper: 2–3 sentences from the task description — who is involved, what's blocking, what done looks like. Smart Brevity tone: bold key terms, no adverbs.
- Varys whisper (if attached): render inline after go-deeper with `📡` prefix. Lede + why. Multiple whispers → one `📡` line each.

**Zheng — compressed:**
- Title + one lede sentence only
- If delegated: `Waiting on: {person}`
- If scheduled: `Due: {date}`
- No why, no go-deeper

**Radar — flat list:**
- Task title prefixed by context label
- No lede, no narrative
- Group by context if more than 5 items

**Omit** any context sub-section with no tasks. Omit any quadrant section with no tasks entirely.

**Completed ONE Things** show ✓. Do not replace with new synthesis. Rolls off after the day changes.

---

## Step 4 — Write and present the report

Write the report to the workspace folder as `{YYYY-MM-DD}-task-report.md`.

Print a summary in the conversation:

```
Task report — {YYYY-MM-DD}
Arena: {N} · Wu Wei: {N} · Zheng: {N} · Radar: {N}
ONE Things: {N} read from TickTick · {N} synthesized today
```

Then present the full report inline.

---

## Design notes

**The report reflects live task state, not the email snapshot.** Tasks created from any source — email, Slack, meetings, manual entry — all appear. Running the report twice in a day shows the updated board.

**ONE Things are set once per day.** The due-date check is the idempotency guard. If today's ONE Thing exists, it is read — not re-generated. A ONE Thing that changes with every report run is reactive noise, not a commitment.

**Completed ONE Things remain visible.** Seeing ✓ is motivating. The completed ONE Thing rolls off only after the day changes.

**Whispers are enrichment, not tasks.** They appear inline under the task they enrich — not in their own report section.

**Report is per-day, not per-session.** Multiple triage runs in a day (email, meeting, Slack) all feed the same TickTick board. The report reflects the board — not the last source ingested.

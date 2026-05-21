---
name: inbox-triage
description: >
  Stage 4 of the anansi triage pipeline. Reads an sb-atomize atomized file,
  extracts action items from Discussion and Project blocks, infers Pakele-OS
  context (me/dcs/iq/ai/anykine) and Zero-Null quadrant (arena/wuwei/zheng/radar)
  for each, then creates or updates tasks in TickTick. Attaches Varys whispers
  to relevant action items as enrichment notes. Orphan whispers route to Anansi
  via anansi_capture. Deduplication: matching task found in TickTick appends a
  timestamped update note to description instead of creating a duplicate.
  Triggers: "triage the atomized output", "run inbox-triage", "/inbox-triage",
  "classify action items", "push tasks to ticktick", "route action items",
  "triage this atomized file", "classify and route tasks".
argument-hint: "[atomized file path]"
---

# inbox-triage

Stage 4 of the anansi pipeline. Upstream stages identify and atomize. This skill classifies and routes — turning action items from the atomized output into TickTick tasks, enriched with Pakele-OS context labels, Zero-Null quadrant priority, and any Varys whisper context that applies.

---

## When to invoke

Invoke when:
- An `sb-atomize` run has produced a `{source-slug}-atomized.md` file
- The user says "triage the atomized output", "/inbox-triage", "classify action items", "push tasks to ticktick", "route action items", "triage this atomized file"
- Downstream of the inbox pull → para-process → sb-atomize pipeline

Do not invoke:
- Before `sb-atomize` has run — this skill reads the atomized file, it does not produce it
- As a standalone tool without an atomized input file

---

## Inputs

One file: `{source-slug}-atomized.md` produced by `sb-atomize`.

---

## Step 1 — Read the atomized file

Read the entire atomized file end-to-end. Extract:

**Action items** from:
- Section 3 Discussion blocks → `### Action Items` bullets (shape: `{Owner} to {do X} by {date}`)
- Section 1 Project blocks → explicit next steps in `### Narrative` or implied by completion criteria
- Any `task` template entity blocks if present in Section 4

**Varys whispers** — collect all `w.N` blocks with their `## Edges` entries. These will be matched to action items in Step 4.

For each action item, record:
- Raw action text
- Source block address (e.g., `3.2`) and entity slugs from that block's `## Edges`
- Owner (if named) and due date (if stated)
- Project/area edges (`under_project:`, `under_area:`) for context inference
- The source block's lede and why sentences for task description seeding

---

## Step 2 — Infer Pakele-OS context

Assign one of five contexts to each action item. Use source block entities, edges, and the action text as signals.

| Context | Signals |
|---|---|
| `dcs` | POW, Puʻuhonua, DCS, Dynamic Community Solutions, board, grant, nonprofit, DCCA, village, Waiʻanae, Kendall, Luminate, accumulus, Trinity, Ron |
| `iq` | JERA, iQ360, LNG, liquefied natural gas, Lori, Lynn, Casey, HECO, Coalition for Hawaii's Energy Future, energy, Par Pacific |
| `ai` | Pakele.ai, AI tools, Claude, prompt, model, API, automation, consulting (non-JERA) |
| `me` | family, health, keiki, school, home, personal, surf, music, recreation, groceries, CSA, insurance, personal finance |
| `anykine` | ConCon, Kahoolawe, civic engagement outside DCS, personal community projects with ongoing meetings and responsibility |

**Inference rules:**
- DCS umbrella or POW brand → `dcs`, even if James is acting personally
- JERA or iQ360 contract work → `iq` regardless of framing
- James engaging as a community member (not in nonprofit role) in substantive civic work → `anykine`
- Surf alerts, casual personal emails → `me`, never `anykine`
- `anykine` test: is this a substantive civic project with meetings, actions, and ongoing responsibility — that isn't DCS/POW, isn't AI consulting, and isn't purely personal life? If yes → `anykine`
- When genuinely ambiguous, pick the stronger signal and append `[context: inferred from {signal}]` in the task description

---

## Step 3 — Assign Zero-Null quadrant

For each action item, assign one quadrant based on importance and urgency.

| Quadrant | Label | Condition | TickTick Priority |
|---|---|---|---|
| Arena | `arena` | Important + due ≤ 48h or overdue | 5 (High) |
| Wu Wei | `wuwei` | Important + due > 48h or undated | 3 (Medium) |
| Zheng | `zheng` | Not James's to drive — delegated, waiting, or scheduled | 1 (Low) |
| Radar | `radar` | Monitor or reference — no near-term action | 0 (None) |

**Importance signals:** James is the direct owner, it blocks a project, it has a named funder or hard deadline, it's a clear ONE Thing candidate.

**Urgency signals:** explicit due date ≤ 48h, overdue, "ASAP" or equivalent, on a critical path with imminent downstream consequence.

**Delegation/Zheng signals:** owner is not James, action is "waiting on {person}", James's role is to monitor or check in rather than execute. Zheng can be urgent in clock time but is not James's work to do.

When due date is absent and urgency signals are absent: default to `wuwei` if important, `radar` if not.

---

## Step 4 — Attach Varys whispers

For each action item, check whether any `w.N` whisper block has an `## Edges` entry connecting to the same entity as the action item.

**Match criteria (any of):**
- Whisper edge slug matches an entity slug in the action item's source block `## Edges`
- Whisper edge slug matches the project or area the action item is `under_project:` / `under_area:`
- Whisper and action item share the same source block address

If a match: attach whisper lede + why to the task description as a signal note (see Step 5 format).

**Orphan whispers** — whispers with no matching action item — route to Anansi via `anansi_capture` using the `entity-note` template. Do NOT create TickTick tasks for orphan whispers. Whispers are knowledge, not work.

---

## Step 5 — Create or update TickTick tasks

For each classified action item:

### Deduplication check

Search TickTick using the first 6–8 significant words of the action item title.

- **High-confidence match** (same title shape, same context project): **update** — append a timestamped note block to the task's content/description field.
- **Low-confidence or no match**: **create** new task.

Prefer creating a new task over incorrectly merging. A wrong merge loses accumulated context.

### Task fields

| Field | Value |
|---|---|
| Title | Action item text, verb + object format, ≤60 chars if possible |
| Project/List | TickTick project matching context (`dcs` / `iq` / `ai` / `me` / `anykine`). Create list if not present. |
| Priority | Per quadrant mapping above |
| Due date | From action item if stated. Omit if absent. |
| Tags | Quadrant label: `arena` / `wuwei` / `zheng` / `radar` |
| Content | Task description (see below) |

### Task description format

**New task:**

```
{Source block lede — one sentence, the core action or outcome.}

**Why it matters:** {Source block Why sentence.}

**Context:** {2–3 sentence Smart Brevity digest of the relevant Narrative — who is involved, what's at stake, what done looks like.}

**Completion criterion:** {What done looks like, if inferable from the source block.}

[If whisper attached:]
📡 Signal: {Whisper lede — one sentence, ≤15 words.}
↳ {Whisper why — one sentence, ≤20 words.}
```

**Updated task (append to existing description):**

```
---
[{YYYY-MM-DD} update]
{New context from this triage run — 1–3 sentences. What changed, what's new.}
[📡 Signal: {whisper lede} — if a new whisper applies to this update.]
```

---

## Step 6 — Summary output

After all tasks are created and updated, print a triage summary:

```
Inbox triage complete — {source-slug} — {YYYY-MM-DD}

Tasks created:  {N}  (arena: {N} · wuwei: {N} · zheng: {N} · radar: {N})
Tasks updated:  {N}
Whispers → tasks: {N} attached
Whispers → Anansi: {N} orphans routed

Context breakdown:
  dcs: {N} · iq: {N} · ai: {N} · me: {N} · anykine: {N}
```

---

## Design notes

**Whispers enrich, they don't create tasks.** A Varys whisper is knowledge context that changes how everything should be read. It attaches to the action item it enriches. Orphan whispers go to Anansi as knowledge nodes. Nothing is lost.

**Deduplication is conservative.** The skill prefers a new task over a wrong merge. Accumulated context in a task's description field is valuable — overwriting it with a mismatched update is worse than a brief duplicate.

**Zheng is about role, not timing.** A task is Zheng when James is not the primary actor — even if the clock is urgent. The action is to monitor, follow up, or check in, not to execute.

**Context inference is one-pass.** Make a call, note the inference signal in the task description if ambiguous, and move on.

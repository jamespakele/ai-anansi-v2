---
name: inbox-triage
description: >
  Reads an sb-atomize atomized file produced from an email pull, extracts action
  items from Discussion and Project blocks, infers Pakele-OS context
  (me/dcs/iq/ai/anykine) and Zero-Null quadrant (arena/wuwei/zheng/radar) for
  each, attaches matching Varys whispers, routes orphan whispers to Anansi via
  anansi-atom, then hands the fully classified item list to
  r2-task-manager:triage-ingest for deduplication and TickTick creation.
  Supersedes r2v2:inbox-triage and anansi:inbox-triage. Triggers: "triage
  the atomized output", "run inbox-triage", "/inbox-triage", "classify action
  items", "route action items", "triage this atomized file", "classify and route
  tasks", "push to ticktick", "triage email action items".
argument-hint: "[atomized file path]"
---

# inbox-triage

Classifies and routes action items from an atomized email pull file. Steps 1–4
live here: read, classify context, assign quadrant, attach whispers. Step 5
(deduplication + task creation) is delegated to `r2-task-manager:triage-ingest`
— this skill never calls TickTick directly.

---

## When to invoke

Invoke when:
- `r2-remember` has fully processed an `inbox-{date}.md` file through the
  atomization pipeline, producing a `{source-slug}-atomized.md` file
- The user says "triage the atomized output", "/inbox-triage", "classify action
  items", "route action items", "triage this atomized file"

Do not invoke:
- Before `sb-atomize` has run — this skill reads the atomized file, it does not
  produce it
- Without an atomized input file

---

## Step 1 — Read the atomized file

Read the entire atomized file end-to-end. Extract:

**Action items** from:
- Section 3 Discussion blocks → `### Action Items` bullets (shape: `{Owner} to {do X} by {date}`)
- Section 1 Project blocks → explicit next steps in `### Narrative` or implied by completion criteria
- Any `task` template entity blocks if present in Section 4

**Varys whispers** — collect all `w.N` blocks with their `## Edges` entries.
These will be matched to action items in Step 4.

For each action item, record:
- Raw action text
- Source block address (e.g., `3.2`) and entity slugs from that block's `## Edges`
- Owner (if named) and due date (if stated)
- Project/area edges (`under_project:`, `under_area:`) for context inference
- The source block's lede and why sentences for task description seeding

---

## Step 2 — Infer Pakele-OS context

Assign one of five contexts to each action item using source block entities,
edges, and action text as signals.

| Context | Signals |
|---------|---------|
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
- `anykine` test: substantive civic project with meetings, actions, and ongoing responsibility — not DCS/POW, not AI consulting, not purely personal? → `anykine`
- When genuinely ambiguous: pick the stronger signal and append `[context: inferred from {signal}]` in the task description

---

## Step 3 — Assign Zero-Null quadrant

For each action item, assign one quadrant based on importance and urgency.

| Quadrant | Label | Condition | Priority |
|----------|-------|-----------|----------|
| Arena | `arena` | Important + due ≤ 48h or overdue | High |
| Wu Wei | `wuwei` | Important + due > 48h or undated | Medium |
| Zheng | `zheng` | Not James's to drive — delegated, waiting, scheduled | Low |
| Radar | `radar` | Monitor or reference — no near-term action | None |

**Importance signals:** James is the direct owner, it blocks a project, it has
a named funder or hard deadline, it's a clear ONE Thing candidate.

**Urgency signals:** explicit due date ≤ 48h, overdue, "ASAP" or equivalent,
on a critical path with imminent downstream consequence.

**Delegation/Zheng signals:** owner is not James, action is "waiting on
{person}", James's role is to monitor or check in rather than execute. Zheng
can be urgent in clock time but is not James's work to do.

When due date and urgency signals are both absent: default to `wuwei` if
important, `radar` if not.

---

## Step 4 — Attach Varys whispers

For each action item, check whether any `w.N` whisper block connects to the
same entity.

**Match criteria (any of):**
- Whisper edge slug matches an entity slug in the action item's source block `## Edges`
- Whisper edge slug matches the project or area the action item is `under_project:` / `under_area:`
- Whisper and action item share the same source block address

If matched: attach whisper lede + why to the item's task description payload
as a signal note (see format in `r2-task-manager:triage-ingest`).

**Orphan whispers** — whispers with no matching action item — route to Anansi
via `anansi:anansi-atom` using the `entity-note` template. Do NOT create
tasks for orphan whispers. Whispers are knowledge, not work.

---

## Step 5 — Hand off to r2-task-manager:triage-ingest

Assemble the classified item list and invoke `r2-task-manager:triage-ingest`.

Pass for each item:
- `title` — action text, verb + object format, ≤60 chars if possible
- `context` — one of: `dcs` / `iq` / `ai` / `me` / `anykine`
- `quadrant` — one of: `arena` / `wuwei` / `zheng` / `radar`
- `due_date` — ISO 8601 if stated, omit if absent
- `lede` — source block lede sentence
- `why` — source block why sentence
- `narrative` — 2–3 sentence Smart Brevity digest of the relevant context
- `completion_criterion` — what done looks like, if inferable
- `whisper` — attached whisper lede + why, if matched (omit if none)
- `source_slug` — the atomized file slug, for provenance

`triage-ingest` handles deduplication, task creation, and the summary report.

---

## Step 6 — Report orphan whisper routing

After handing off to `triage-ingest`, report what was sent to Anansi:

```
Orphan whispers → Anansi: {N} routed
```

If zero orphans, omit this line. The full triage summary comes from
`triage-ingest`.

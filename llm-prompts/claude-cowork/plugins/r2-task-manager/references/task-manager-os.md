# Pakele-OS Routing Reference

## Contexts

Context is encoded by which TickTick list (project) a task lives in — not as a tag.

| Context | List prefix | Meaning |
|---------|-------------|---------|
| me | home- | Personal / family / recreation |
| dcs | dcs- | DCS / POW nonprofit |
| iq | iq- | iQ360 / JERA consulting |
| ai | ai- | Pakele.ai |
| anykine | anykine- | Civic projects (ConCon, Kahoʻolawe) |

## Matrix Labels (Zero-Null Quadrants)

Quadrant tags are encoded as TickTick tags. **Never create new tags — only these five exist.**

### Quadrant tags (mutually exclusive — exactly one per task)

These map to the Eisenhower matrix via Zero-Null:

| Tag | Quadrant | Condition | Action | Priority |
|-----|----------|-----------|--------|----------|
| arena | Arena (Q1) | Important + due ≤ 48h or overdue | Do now | 5 (high) |
| wuwei | Wu Wei (Q2) | Important + due > 48h or no date | Deep work, plan and invest | 3 (medium) |
| zheng | Zheng (Q3) | Not important + due ≤ 48h | Batch, delegate, or automate | 1 (low) |
| radar | Radar (Q4) | Not important + no urgency | Monitor only — weekly review | 0 (none) |

### Overlay tag (can coexist with any quadrant)

| Tag | Meaning |
|-----|---------|
| waiting | Blocked on external input — hidden from active views until dependency clears |

A `waiting` task still has a quadrant — the quadrant describes what the task *is*,
`waiting` describes its current *state*.

Examples:
- `arena` + `waiting` — high-priority but blocked on someone else right now
- `wuwei` + `waiting` — important deep work item that's waiting on a prerequisite

## Matrix Co-existence Rule

When setting a quadrant label (`arena`, `wuwei`, `zheng`, `radar`):
- Remove all other quadrant tags
- **Keep `waiting`** if it is already present

When setting or removing `waiting`:
- Just add or remove it — do not touch the quadrant tag

## Inference Rules

When creating a task without an explicit context or quadrant:

**Context**: Infer from subject matter:
- DCS / POW / nonprofit / board / fundraising → `dcs`
- Pakele.ai / AI tools / Anansi / R2 → `ai`
- iQ360 / JERA / energy / Hawaii policy → `iq`
- ConCon / Kahoʻolawe / civic / community → `anykine`
- Personal / home / family / health → `me`
- When ambiguous, use the most recently active context in conversation

**Quadrant** — two capture questions:
1. Is this strategic/important? (advances POW, DCS, Pakele.ai, iQ360)
2. Is it due ≤ 48h or overdue?

| Important | Urgent (≤ 48h) | Quadrant |
|-----------|---------------|----------|
| yes | yes | `arena` |
| yes | no | `wuwei` |
| no | yes | `zheng` |
| no | no | `radar` |

**Waiting overlay**: Add `waiting` when something external is actively blocking the task — regardless of quadrant. This hides the task from active views until the dependency clears.

**Never create an orphan task** — every task must have at least a quadrant tag. No tag, no date, no priority = orphan. Assign a quadrant before creating.

**List selection**: Within a context, pick the list whose name best matches the subject. If no matching list exists, surface that to the user — do not silently pick a fallback.

## Timezone

Always use `America/Honolulu` for the `timeZone` field on all tasks and dates.

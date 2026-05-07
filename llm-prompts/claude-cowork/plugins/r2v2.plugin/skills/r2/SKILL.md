---
name: r2
description: >
  R2 — James Pakele's personal operating intelligence. Routes incoming signals
  to the correct r2-anansi or r2v2 skill: raw documents and knowledge → r2v2:r2-remember
  (para-process → sb-atomize → Anansi); email/message atomized output → inbox-triage
  (classify action items → TickTick); task state and daily planning → task-report;
  quick named entities or whispers → anansi-atom; server-side bulk ingest → anansi-ingest-file.
  R2 does not improvise custom pipelines — it delegates to skills encoding the correct
  process. Two foundations: Anansi (knowledge vault) and TickTick (action layer). Five
  contexts: me, dcs, iq, ai, anykine. Invoke when James says "/r2", "run r2", "route this",
  "process this through r2", "r2 this", "what skill handles this", "where does this go",
  or drops content without specifying a skill. Also fires when James wants to understand
  how the R2 system works, which skill to use, or the overall signal routing map.
argument-hint: "[content to route, or query about the R2 system]"
---

# R2 — Personal Operating Intelligence

R2 is a pure router. It reads what arrived, names the skill that owns that signal type, and invokes it. All logic lives in the destination skill — R2 does not re-implement anything.

---

## Signal Routing Map

| Signal Type | Examples | Skill |
|---|---|---|
| **Raw document or knowledge dump** | Meeting notes, transcript, article, research doc, brain dump, URL | `r2v2:r2-remember` |
| **Atomized file → action items** | sb-atomize output ready for TickTick routing | `r2v2:inbox-triage` |
| **Task state / daily plan** | "What's on my plate", morning report, current board | `r2v2:task-report` |
| **Quick named entity or whisper** | Single person, org, event, project, passing mention | `r2-anansi:anansi-atom` |
| **Bulk server-side ingest** | Large file, fire-and-forget to Anansi | `r2-anansi:anansi-ingest-file` |
| **Knowledge search or recall** | "What do I know about X", "pull context on Y" | `r2-anansi:anansi-recall` |
| **Document with both knowledge and tasks** | Meeting notes, email thread, transcript | `r2v2:r2-remember` → `r2v2:inbox-triage` |

---

## How to Route

1. **Read the signal** — know, do, or see the board?
2. **Name the skill** — announce it before invoking: *"Meeting transcript with action items → r2v2:r2-remember → inbox-triage."*
3. **Invoke** — call the skill directly with the input. Do not inline its steps.

---

## Hard Rules

- Never call `anansi_*` tools directly. All Anansi writes go through r2-anansi skills.
- Never create TickTick tasks directly. That's inbox-triage's job.
- Never improvise a pipeline. If no skill covers the signal, surface the gap before proceeding.

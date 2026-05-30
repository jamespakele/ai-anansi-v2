# Cross-validation pass — concon

Second-source pass against `output/concon/`. The broadband-hui run hit the empty-PA case; this one exercises the populated-PA path. Together they prove the routing, threading, and Varys rules hold across both shapes.

This isn't a full atomization. It walks the parts of the run that test rules the broadband-hui example didn't.

---

## What concon stresses that broadband-hui didn't

| Feature | broadband-hui | concon |
|---|---|---|
| `## 1. Projects` | `_(none found)_` | 4 top-level + 2 sub-projects |
| `## 2. Areas` | `_(none found)_` | 1 Area |
| Sub-project nesting (`1.2.1`, `1.2.2`) | none | yes — GIA Application + Filmmaker Meet-and-Greet under Documentary |
| `[area:2.1]` cross-reference tag in TOC | n/a | yes — every Project carries it |
| `**Address:** 1.X` field in typed file | n/a | yes |
| `**Parent project:** 1.2` in typed file | n/a | yes (sub-projects) |
| `**Parent area:** 2.1` in typed file | n/a | yes (every Project) |
| `## Notes` "borderline candidates" with PA populated | n/a | yes (Statewide Data Analysis, Vision Doc, Tiffany topic) |

---

## Rule 1 — Sequential processing holds

Order remains: PA → Resources → Discussion → Concepts → Varys.

For concon, this means:

- `### 1.1` (Political-Entity Comparison Matrix) renders first.
- `### 1.2` (HIPA ConCon Documentary Film) renders before its sub-projects.
- `### 1.2.1` (GIA Application) and `### 1.2.2` (Filmmaker Meet-and-Greet) render after their parent.
- `### 2.1` (HIPA ConCon Strategy & Coalition Coordination) is the only Area.
- Section 3 (Discussion) shards reference all of the above via `under_project: project:slug` and `under_area: area:hipa-concon-strategy` edges.

Sub-project example block (1.2.1):

```
### 1.2.1 GIA Application for ConCon Documentary [project]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | confidence: high | vault_match_hint: project:gia-application-concon-documentary -->
HIPA submitted a Grant-in-Aid request to the Hawaiʻi legislature to fund the ConCon documentary project, with a decision expected at session-end.

**Why it matters:** The GIA outcome is the funding pivot — a yes shifts production timeline, a no forces an alternate-funder push (Omidyar, etc.).

## Identity
- **Goal:** secure Grant-in-Aid funding from the Hawaiʻi legislature for the ConCon documentary project
- **Status:** Active
- **End date:** end of 2026 legislative session

## Summary
HIPA's GIA application is in front of the legislature this session, with imminent budget release expected the night of the May 2 meeting.

## Content
The application has been read by Kyle Yamashita and is being weighed against the legislature's overall GIA appropriation. Richmond and Perry led the narrative work. The HIPA team treats the result as a fork — a yes accelerates documentary production toward the late-2027 / Q1-2028 release window; a no pivots fundraising to DC-based democracy funders led by Omidyar Network. Either way, the result feeds the C4 entity-form decision in June.

## Edges
- sub_project_of: project:hipa-concon-documentary-film
- under_area: area:hipa-concon-strategy
- references: note:hipa-gia-application
- contributors: person:richmond
- contributors: person:perry

---
```

The `sub_project_of:` edge and the `under_area:` edge resolve to slugs already in the same atomized file (1.2 and 2.1 respectively). Sequential ordering makes that resolution mechanical.

---

## Rule 2 — Reviewer notes survive

`resources-typed.md` for concon doesn't carry `_Reviewer note:_` lines on most resources, but the **PA `## Notes` rationale** carries five "borderline calls the user should confirm" plus several "considered and dropped" candidates.

Per the entity-routing rules: when PA is **populated**, these don't get a standalone audit block. They surface as Reviewer-note flags inside the closest related block. Examples:

- "Statewide District-Profile Data Analysis was considered as a Project and dropped" → goes as a `## Review` line on the closest related block, which is `### 1.3 Delegate Roster Brainstorm Spreadsheet [project]` (Richmond proposed both in the same brainstorm). The reviewer note: `**Reviewer note (sb-atomize):** Statewide District-Profile Data Analysis was considered as a sibling project but dropped per the precision rule (no deadline, no owner). Re-evaluate next ingest if owner + date appear.`
- "Vision Document for ConCon was considered and dropped" → `## Review` on the discussion block `### 3.11 Vision document gap` since that's where the conversation lived. The reviewer note: `**Reviewer note (sb-atomize):** Vision Document was considered as a Project but dropped — pure aspirational framing, no owner, no deadline. Promote if HIPA assigns an owner with a target outcome.`
- "Bill/Perry Legislative Session work was considered as an Area and dropped" → `## Review` on `### 4.1 Bill [person]` since that's the most directly affected entity.

This is the entity-routing rule kicking in: borderline content surfaces, but on the closest related entity rather than as a standalone audit.

---

## Rule 3 — Type confidence threading

concon has medium-confidence persons (most of the HIPA core team are first-name only — `person:bill`, `person:perry`, `person:michael`, `person:richmond`, `person:lauren`). Each gets `confidence: medium` in its block metadata comment.

```
### 4.1 Bill [person]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | confidence: medium | vault_match_hint: person:bill -->
Bill is a HIPA core team member experienced with Hawaii ballot initiatives.

## Met via
HIPA ConCon coalition — set up and ran the COBRA-affiliated affordable-housing ballot committee 6–8 years ago.

## Review
**Reviewer note (sb-atomize):** Bill/Perry Legislative Session work was considered as an Area and dropped — referenced repeatedly as availability constraint but no HIPA-collective standard around it.

---
```

No `<!-- review-needed: low confidence -->` tag because confidence is medium, not low. Cowork-style downstream filtering would catch this on the medium tier rather than escalating it.

---

## Rule 4 — Whisper detection on concon

The concon transcript is rich in whisper material — much richer than broadband-hui — because the conversation is consciously multi-track (entity-form mechanics, donor concerns, legislative timing, personal-liability worries, candidate-pool constraints). Sample whispers Varys would catch:

### w.1 — orphaned game-changer

```
### w.1 "C4 keeps us out of jail" framing [whisper]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | whisper_category: orphan | confidence: medium | vault_match_hint: whisper:c4-jail-framing -->
**Lede:** The team frames C4 selection as a personal-criminal-liability question, not just compliance.

**Why it matters:** Reframes the entity-form decision from policy to risk — affects who is willing to serve as treasurer.

**Source quote:** "The C4 is going to be the easiest way for us to not be in jail."

## Edges
- concerns: project:stand-up-c4-continuity-entity
- concerns: project:political-entity-comparison-matrix
- related_to: organization:hawaii-campaign-spending-commission

---
```

This *is* mentioned in the typed-file Evidence for project 1.4, but the *risk-framing implication* — that the choice is being made on personal-liability grounds — never lands as the explicit driver in any block. That's the whisper.

### w.2 — relationship nuance

```
### w.2 Donors prefer C4 anonymity [whisper]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | whisper_category: nuance | confidence: high | vault_match_hint: whisper:donor-anonymity-preference -->
**Lede:** Some HIPA donors specifically want to "stay off the radar" via C4 structure.

**Why it matters:** Donor preference is now a constraint on entity form — overrides pure-strategy framing in the Matrix project.

**Source quote:** "It was a C4 those donors can actually have stay off the radar too."

## Edges
- concerns: project:political-entity-comparison-matrix
- concerns: project:stand-up-c4-continuity-entity

---
```

### w.3 — verbal decision

```
### w.3 Treasurer is the high-liability seat [whisper]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | whisper_category: decision | confidence: high | vault_match_hint: whisper:treasurer-liability-seat -->
**Lede:** Group acknowledges the treasurer carries personal criminal liability for filings.

**Why it matters:** Recruiting a treasurer is now a precondition for any entity stand-up — affects the C4 timeline.

**Source quote:** "That's the one that goes to jail if anything goes wrong... someone who's, like, extremely meticulous and like, is, like, willing to put their..."

## Edges
- concerns: project:stand-up-c4-continuity-entity
- concerns: project:political-entity-comparison-matrix

---
```

### w.4 — pattern (legislative)

```
### w.4 Donovan eyeing LG or mayor run [whisper]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | whisper_category: pattern | confidence: medium | vault_match_hint: whisper:donovan-positioning -->
**Lede:** Donovan has telegraphed he is considering an LG or mayor run.

**Why it matters:** Positioning explains his Tiffany handling — the freeze on enabling legislation is electoral, not policy.

**Source quote:** "He's basically telegraphed that he's considering running for LG or mayor."

## Edges
- related_to: person:donovan-dela-cruz
- concerns: note:tiffany-tif-constitutional-amendment

---
```

### w.5 — absence

```
### w.5 No filmmaker named yet [whisper]
<!-- source_id: e5ad138d2c09 | generated_at: 2026-05-02T12:00:00-10:00 | whisper_category: absence | confidence: high | vault_match_hint: whisper:no-named-filmmaker-may-2 -->
**Lede:** As of May 2, no filmmaker has been formally named for the documentary.

**Why it matters:** Anthony is interested but unconfirmed — film-festival submission deadlines (HIFF June 5 2027) require a locked filmmaker upstream.

**Source quote:** "We should at least have a stretch goal of having our filmmaker locked in."

## Edges
- concerns: project:hipa-concon-documentary-film
- concerns: project:filmmaker-meet-and-greet-anthony

---
```

These five whispers, as a set, demonstrate accumulation: each one is small, but together they describe a coherent operational picture (the team is gating C4 stand-up on liability comfort + donor preference + treasurer recruitment, while documentary timeline depends on filmmaker confirmation, while legislative-track work is shaped by Donovan's positioning) that no single entity block captures.

---

## Divergences worth noting

Both runs preserve the same upstream contract (frontmatter shape, TOC structure, typed entity blocks). Two structural differences the atomizer absorbs cleanly:

1. **populated PA adds `**Address:**`, `**Parent area:**`, and `**Parent project:**` fields to the typed entity blocks.** The atomizer maps these into block headings (`### 1.2.1` shows nesting at the address level) and into edges (`under_area:`, `sub_project_of:`).

2. **populated-PA borderline candidates surface as Reviewer notes on the closest related block, NOT as an audit block.** The audit block is exclusively for the empty-PA case. broadband-hui has it; concon does not.

No other divergences. The same routing rules, same metadata threading, same Varys hard caps apply across both runs.

---

## What this proves

Running the skill mentally against both broadband-hui and concon confirms:

- **Empty-PA path:** clean (single audit block at 0.0, skip Sections 1+2, proceed to 3+4).
- **Populated-PA path:** clean (sub-project nesting via address, parent-area edges, borderline reviewer-note surfacing).
- **Sub-project + parent-area cross-references** resolve to slugs that exist in the same output (sequential ordering pays off).
- **Reviewer-note threading** survives in both shapes.
- **Confidence threading** captures the medium-confidence person clusters in concon and the high-confidence-everywhere shape of broadband-hui.
- **Varys stop criteria** differ between sources: broadband-hui yields ~3 strong whispers, concon yields ~5. Both are within the "three-to-five sharp whispers beats thirty noisy ones" range. Neither requires bending the hard caps.
- **Concept handling** is identical (closing line, no per-concept blocks).

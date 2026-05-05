# Entity-routing rules

Information always flows to the entity it belongs to. Never duplicated. When two locations seem plausible, ask: *what kind of fact is this?* Identity → goes on the identity entity. Activity → goes on the activity entity. Container → goes on the container.

These rules apply universally. They are non-negotiable. They are what prevent the most common atomization failure mode: leaky entities that all redundantly describe the same situation.

---

## Rule 1 — Person from a meeting

**Person block holds identity only.** Name, contact email, contact phone, met_via.

What the person said → discussion block (Section 3) `## Content`.
What the person decided → discussion block `## Decisions`.
What the person committed to do → discussion block `## Action Items`.
What organization the person is affiliated with → discussion block edges + organization block `## Edges`.

**Lede shape:** `{Name} is {single identifying noun phrase}.` Nothing more.

Example — wrong:
```
### 4.8 Jaren (DHHL) [person]
Jaren is the Broadband Program Administrator at DHHL who presented the
DHHL Use & Adoption RFP, awarded to ULU High Tech, with first cohort
planned for Molokai in summer.
```

Example — right:
```
### 4.8 Jaren (DHHL) [person]
Jaren is the Broadband Program Administrator at the Department of Hawaiian Home Lands.

## Met via
Broadband Hui #268 — presented the DHHL Use & Adoption RFP update.
```

The RFP, ULU High Tech, and Molokai cohort all live in their own blocks. The person block does not duplicate them.

---

## Rule 2 — Person from a book

**Person block from a book context holds a one-line who-they-are blurb.** The lede is the identifying noun phrase. The "why this person appears in the book" goes in `## Met via` (or `## Content` if the template is `entity-note`-shaped).

Example:
```
### 4.6 Philip Dufresne [person]
Philip Dufresne is a former CIA President's Daily Brief author who became
an Axios staffer.

## Met via
Cited in *Smart Brevity* as the bridge between intelligence-community
brevity discipline and Axios-style journalism.
```

What he said about brevity → goes in the chapter block (Section 3, e.g., `### 3.2.2 Ch. 2: Smart Brevity, Explained [book_chapter]`).

---

## Rule 3 — Organization

**Organization block holds org identity.** Name, full_name, type, domain. Plus a `## Context` section of additive orphaned facts that don't deserve their own atomic note (donations, moves, milestones).

People who work there → edges only, never in the body. `## Edges: affiliated_with: person:slug`.
Projects the org is doing → those go on the project block in Section 1, not on the org block.
Contracts the org has → those go on the relevant project / event / discussion block, not on the org block.

Example — wrong:
```
### 4.19 ULU High Tech [organization]
ULU High Tech is the contractor selected by DHHL to run Phase 1 of the
Use & Adoption RFP, which covers planning, community engagement, program
design, evaluation, and reporting, with a workforce study currently
underway and a first Molokai cohort planned for summer.
```

Example — right:
```
### 4.19 ULU High Tech [organization]
ULU High Tech is a vendor in Hawaii's broadband use-and-adoption program ecosystem.

## Identity
- **Type:** contractor / vendor
- **Domain:** broadband use-and-adoption program (DHHL awardee)

## Context
- Awarded the DHHL Use & Adoption RFP in early 2026.

## Edges
- contracted_by: organization:dhhl
```

The Phase 1 scope, workforce study, and Molokai cohort all belong on the discussion block (`### 3.2`) and the project / note for the RFP itself.

---

## Rule 4 — Topic-discussion

**Discussion block holds the substance of the conversation.** Participants who spoke. Orgs mentioned. Content (what was said, not who said it). Decisions. Action items.

Person identity facts → person block.
Org identity facts → organization block.
Named documents / programs / events referenced → note block.

The decimal address `3.N` is the canonical location for this conversation's content. Other blocks edge into it, not the other way around.

Example — right:
```
### 3.2 DHHL Use & Adoption RFP update from Jaren [discussion]
The DHHL Broadband Program awarded its Use & Adoption RFP to ULU High
Tech and is now in Phase 1 — planning, workforce study, and curriculum
design ahead of a Molokai cohort in summer 2026.

**Why it matters:** This is the use-and-adoption half of DHHL's broadband
program — the digital-literacy and workforce-development counterpart to
infrastructure deployment, with downstream implications for cohort
replication on other islands.

## Participants
- person:jaren-dhhl
- person:burt-lum
- person:sean-mclaughlin

## Organizations
- organization:dhhl
- organization:ulu-high-tech
- organization:nanakuli-high-school

## Content
The RFP dropped in December 2025; ULU High Tech was selected as awardee.
Phase 1 covers planning, community engagement, program design,
implementation support, evaluation, and reporting. A workforce study
is in progress to align training with employer demand. The first cohort
is planned for Molokai in summer 2026, contingent on curriculum
readiness. Nanakuli High School interns helped shape the foundational
curriculum, which centers basic digital literacy — typing, email,
navigation — with youth as primary trainers and Kupuna as downstream
beneficiaries.

## Decisions
- Use & Adoption RFP awarded to ULU High Tech.
- First cohort sited in Molokai for summer 2026.

## Action Items
- DHHL to publish needs-assessment results on dhhlbroadband.com once data is finalized.
- DHHL to conduct site visits over summer to verify coverage gaps.

## Edges
- presented_by: person:jaren-dhhl
- references: note:dhhl-use-and-adoption-rfp
- references: note:dhhl-broadband-website
- mentioned_org: organization:ulu-high-tech
- mentioned_org: organization:nanakuli-high-school

---
```

---

## Rule 5 — Concept

**Concepts are not blocks.** They thread into the closing `<!-- concepts: ... -->` line. Each `## Concepts` entry from `resources-toc.md` becomes one hashtag in that line.

Why no per-concept blocks: concepts in this pipeline are flat tags, not first-class entities. The atomized output's parent source-note picks them up via the closing concepts line at ingest time.

If the user wants a concept promoted to a first-class `note`, they re-ingest with `add template` or use `anansi-atom` directly. That's outside this skill's scope.

---

## Rule 6 — Edges section

**Every emitted block (except `[person]`) has a `## Edges` section when relationships are inferable.**

- Person blocks: NO `## Edges`. Persons are never convergence points; relationships surface through the blocks of the things they're connected to. (Canonical contract — see `sb-atomized.md`.)
- Organization blocks: edges for `affiliated_with: person:slug` only. Project/vendor relationships go on the project block.
- Note blocks: edges for `presented_by`, `produced_by`, `under_project`, `references`, `referenced_by`, `managed_by`, `concerns` etc.
- Discussion blocks: edges for `presented_by`, `mentioned_org`, `references`, `under_project`, `under_area`.
- Project blocks: edges for `owner`, `vendor`, `point_of_contact`, `references`, `under_area`.
- Area blocks: edges for `owned_by`, `supported_by`.
- Whisper blocks: edges for `related_to`, `concerns`, `corrects`, `points_at`.

Edge slugs MUST match the exact slug used in the entity's own block heading (which itself matches the `Vault match hint` from the upstream typed file).

If no edges are inferable from the source, omit the `## Edges` section entirely. Do not emit an empty section.

---

## Rule 7 — Reviewer notes from upstream typed files

**`_Reviewer note:_` content survives into a `## Review` section.**

Verbatim. Don't compress. Don't editorialize. Prefix with `**Reviewer note:**`.

Example — `resources-typed.md` says:
```
- _Reviewer note: likely the same entity as person:sara-lin — confirm and merge if so._
```

The atomized block emits:
```
## Review
**Reviewer note:** likely the same entity as person:sara-lin — confirm and merge if so.
```

If the typed entity has multiple `_Reviewer note:_` lines, emit each one as its own `**Reviewer note:**` line under a single `## Review` section.

---

## Rule 8 — Type confidence

**Carry into the metadata HTML comment.** `confidence: low|medium|high`.

If confidence is `low` AND a reviewer note is present, also emit at the top of the block:
```
<!-- review-needed: low confidence -->
```

This lets downstream tooling filter for "weakest links" — entities most likely to be wrong, hallucinated, or duplicates of existing vault records.

---

## Rule 9 — When information could go in two places

The discriminator is *what kind of fact*:

| Fact kind | Goes on |
|---|---|
| Identity (name, contact, type, domain) | Identity entity block |
| Activity (what happened, what was said) | Activity entity block (discussion, exchange, chapter) |
| Relationship (X works for Y, X owns Y) | `## Edges` on the entity where the relationship originates |
| Containing structure (X is part of project Y) | `under_project` / `under_area` edge on the contained block |
| Orphaned org-level fact | Organization `## Context` |

When you genuinely cannot tell which kind of fact it is, surface it as a Reviewer note on the closest related entity rather than picking. The next ingest cycle can resolve.

---

## Rule 10 — Don't invent

If the upstream typed pipeline did not identify an entity, this skill does not promote it. Period.

The temptations to watch for:

- "There's a clearly important thing in the source that didn't get a block." → Whisper or Reviewer note.
- "I can see how the upstream missed it." → Whisper or Reviewer note.
- "It would be weird not to have a block for X." → Whisper or Reviewer note.

The pipeline is fail-closed by design. The Varys pass and Reviewer-note mechanism are the legitimate paths to surface upstream gaps. Don't shortcut them.

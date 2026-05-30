# Worked example — broadband-hui

End-to-end demonstration of `sb-atomize` against `output/broadband-hui/`.

This is the **PA-empty case**. Both `## 1. Projects` and `## 2. Areas` in `projects-areas-toc.md` are `_(none found)_` and `projects-areas-typed.md` carries a `## Notes` rationale block.

The full atomization would emit ~95 blocks (≈10 discussion shards + ~80 resources + concepts + whispers). For brevity this example walks through:

- The opening header
- The PA Audit block (Section 0)
- One discussion shard: **3.2 DHHL Use & Adoption RFP update from Jaren**
- One person block: **4.8 person:jaren-dhhl**
- The concept `#use-and-adoption` (in the closing line)
- One Varys whisper drawn from the transcript

Each section shows the **input slice** that drove the output, then the **emitted block**. This is what the parser sees.

---

## Input recap

From the four pipeline files:

```yaml
source_id: bh268-2026-04-15
generated_at: 2026-05-02T00:00:00-10:00
```

`projects-areas-toc.md`:
```
## 1. Projects
_(none found)_

## 2. Areas
_(none found)_
```

`projects-areas-typed.md` `## Notes`:
> Consumption-shaped document from the user's actionability perspective. This is a transcript of Broadband Hui episode #268... The user (James Pakele) is referenced exactly once — at the very end, when Burt says "I think it's too late for the read. AI Pakele has his notes" — confirming James's read.ai note-taker was in the meeting but James himself is not described as a presenter, contributor, or commitment-holder anywhere... A clean empty PA result is a successful run, not a failure.

`resources-toc.md` `## 3. Discussion`:
```
- 3.2 [discussion] DHHL Use & Adoption RFP update from Jaren — ULU High Tech award, workforce study, Molokai cohort planning, Hawaiian-language website
```

`resources-toc.md` `## 4. Resources` (relevant lines):
```
- 4.8 [person:jaren-dhhl] Jaren (DHHL)
- 4.18 [note:dhhl-use-and-adoption-rfp] DHHL Use & Adoption RFP
- 4.19 [organization:ulu-high-tech] ULU High Tech
- 4.20 [note:dhhl-broadband-website] dhhlbroadband.com
- 4.21 [organization:nanakuli-high-school] Nanakuli High School
- 4.22 [organization:dlir] DLIR
- 4.9  [organization:dhhl] DHHL
```

`resources-typed.md` for `person:jaren-dhhl`:
```
### person:jaren-dhhl
- **Name:** Jaren (DHHL)
- **Template:** entity-person.md
- **Type confidence:** high
- **Identity fields:** name=Jaren; met_via=Broadband Hui #268; Broadband Program Administrator at Department of Hawaiian Home Lands; last name not stated in source
- **Evidence:** "I'm Jaren I'm the Broadband Program Administrator for the Department of Hawaiian Homelands."
- **Concepts:** #dhhl #use-and-adoption #broadband-program
- **Vault match hint:** person:jaren-dhhl
```

`resources-toc.md` `## Concepts` (relevant line):
```
- `#use-and-adoption` — "the digital-literacy and workforce-development half of broadband programs, distinct from infrastructure deployment"
```

Source content slice (`transcript.txt` lines 59–95):
> Burt: "we, we've been talking about the use and adoption at dhhl, came out with an RFP back in the December of last year… I wanted to give Jaren a chance to chat a little bit about, you know, the RFP and what you have in mind to do…"
> Jaren: "I'm Jaren I'm the Broadband Program Administrator for the Department of Hawaiian Homelands. So Bert is right. We did have an RFP for our use in adoption portion of our program that was dropped in December. We finalized our contract and selected awardee. It's ULU High tech… we're currently doing a workforce study… needs assessment is almost complete. Once we get all the data, we will be publishing it on our website. It's@DHHL broadband.com… page is also translated in Hawaiian. We're looking at doing our first cohort in Molokai in the summer… we've had a bunch of interns from Nanakuli High School, and they actually are the ones that kind of created the foundation for this rfp."

---

## Emitted output (excerpt)

```
<!-- anansi-atomize: Broadband Hui #268 — April 15, 2026 | 95 blocks | 2026-05-02 | source_id: bh268-2026-04-15 -->

### 0.0 PA Audit [note]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | confidence: high | vault_match_hint: note:bh268-2026-04-15-pa-audit -->
Broadband Hui #268 is consumption-shaped — James attended via read.ai but holds no projects or areas surfaced in this transcript.

**Why it matters:** Empty PA on this episode is a successful run, not a failure — it confirms the precision rule held against a Hui rich in *other people's* commitments.

## Considered and dropped
- **Jaren / DHHL Use & Adoption work** — owned by Jaren, not James. Resource, not Project.
- **Wendy / Tech Savvy Teens expo and franchise expansion** — owned by Wendy. Out of James's actionability.
- **Amber / AI Safety Week** — owned by HiBO. Pakele.ai-adjacent but no first-person ownership signal from James in this source.
- **Sean / HSAC + NACo + PUC tracking** — Sean's work.
- **Chung / legislative session tracking** — Chung's work.

## Borderline candidates
- **Policy-subgroup huddle (July)** — Burt and Sean floated it but James was not named as a participant. Would upgrade to a James Project on a future ingest if (a) explicit commitment, (b) a date, and (c) a named outcome.

---

### 3.2 DHHL Use & Adoption RFP update from Jaren [discussion]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | confidence: high | vault_match_hint: topic_discussion:bh268-3-2-dhhl-use-and-adoption -->
DHHL awarded its Use & Adoption RFP to ULU High Tech and is now in Phase 1, building toward a Molokai cohort in summer 2026.

**Why it matters:** This is the digital-literacy and workforce-development half of DHHL's broadband program — the counterpart to infrastructure deployment, with cohort replication implications for other islands.

## Participants
- person:jaren-dhhl
- person:burt-lum
- person:sean-mclaughlin

## Organizations
- organization:dhhl
- organization:ulu-high-tech
- organization:nanakuli-high-school
- organization:dlir

## Content
The RFP dropped in December 2025 and ULU High Tech was selected as awardee. Phase 1 covers planning, community engagement, program design, implementation support, evaluation, and reporting. A workforce study is in progress to align training with local employer demand. The first cohort is planned for Molokai in summer 2026, contingent on curriculum readiness. Nanakuli High School interns helped shape the foundational curriculum, which centers basic digital literacy — typing, email, navigation — with youth as primary trainers and Kupuna as downstream beneficiaries. The internship pipeline routes through DLIR.

## Decisions
- Use & Adoption RFP awarded to ULU High Tech.
- First cohort sited in Molokai for summer 2026.
- Program scope set as basic digital literacy with youth-first delivery.

## Action Items
- DHHL to publish needs-assessment results on dhhlbroadband.com once data is finalized.
- DHHL to conduct site visits over summer to verify coverage gaps.
- DHHL open to sharing curriculum and data with counties on request.

## Edges
- presented_by: person:jaren-dhhl
- references: note:dhhl-use-and-adoption-rfp
- references: note:dhhl-broadband-website
- mentioned_org: organization:ulu-high-tech
- mentioned_org: organization:nanakuli-high-school
- mentioned_org: organization:dlir
- under_org: organization:dhhl

---

### 4.8 Jaren (DHHL) [person]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | confidence: high | vault_match_hint: person:jaren-dhhl -->
Jaren is the Broadband Program Administrator at the Department of Hawaiian Home Lands.

## Met via
Broadband Hui #268 — presented the DHHL Use & Adoption RFP update.

---

[... blocks 4.9 through 4.81, then whisper section ...]

### w.1 Policy subgroup is dying [whisper]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | whisper_category: orphan | confidence: medium | vault_match_hint: whisper:policy-subgroup-thinning -->
**Lede:** The Broadband Hui policy subgroup has thinned to near-empty.

**Why it matters:** Without the subgroup, no one tracks legislation between Hui meetings — a key Hui function may quietly lapse.

**Source quote:** "I don't know what happened to Kieran. Kieran sort of just disappeared."

## Edges
- concerns: organization:hawaii-broadband-office
- related_to: person:burt-lum
- points_at: note:hsac-convening-2026

---

<!-- concepts: #broadband-hui #hawaii-broadband-office #dhhl #use-and-adoption #bead-program #digital-literacy #workforce-development #kupuna #youth-mentorship #tech-savvy-teens #franchise-model #ai-safety-week #deepfakes #scam-awareness #leo-satellite #fiber #isp #cable-franchising #cable-policy #cox-charter-merger #maui-cable-franchise #community-mobilization #telecom-regulation #telecom-policy #policy-subgroup #legislative-tracking #pending-nomination #puc #hsac #naco #county-policy #honolulu-mayor #honolulu-city-council #big-island-council #internships #certification #kahaluu #maui #big-island #kauai #molokai #community-services #community-access-television #youth-tech-mentorship #e-recycling #emergency-connectivity #disaster-recovery #typhoon-disaster #weather #cnmi #pacific-territories #indigenous-broadband #hyperscaler-data-centers #internet-policy #contract-tracking #infrastructure-access #commission-docket #manao #wai #ahupuaa #perseverance #homesteads #ai-image-generation #ai-note-taker #starlink #scam-awareness #policy-subgroup -->
```

---

## What this example demonstrates

### Template adherence

- The **person block** for Jaren follows `entity-person.md` strictly: lede is identity-only ("Jaren is the Broadband Program Administrator…"), `## Met via` is one short sentence, no `## Edges`, no `## Contact` section emitted because email and phone are unknown. No what-he-said. No quotes.
- The **discussion block** for 3.2 follows `meeting-topic-discussion.md`: `## Participants` (only those who spoke in this topic, not all attendees), `## Organizations` (orgs touched), `## Content` (3–6 sentences, Smart Brevity tone, no quotes), `## Decisions`, `## Action Items`, `## Edges`.
- The **PA audit block** follows `entity-note.md` with the additional `## Considered and dropped` and `## Borderline candidates` sections, drawn verbatim from the upstream `## Notes` rationale and Smart-Brevity-compressed.

### Smart Brevity rules

- Tease/title ≤6 words ("DHHL Use & Adoption RFP update from Jaren" — fits when the lead-in tag is dropped, and matches the TOC label).
- Lede is one sentence, the takeaway: "DHHL awarded its Use & Adoption RFP to ULU High Tech and is now in Phase 1, building toward a Molokai cohort in summer 2026."
- `**Why it matters:**` is bold, one sentence, adds perspective (cohort-replication implications) without repeating the lede.
- Bullets used wherever 3+ related points exist (Decisions, Action Items, Edges).
- No adverbs, qualifiers, or throat-clearing.
- Bold marks key terms.

### Edges

- Discussion block edges include `presented_by: person:jaren-dhhl` (Jaren drove this topic), `mentioned_org` for orgs touched, `references` for named documents — all using slugs that resolve to other blocks in the same atomized file.
- Person block has NO `## Edges` per canonical contract — Jaren's relationships surface through the discussion block where he presented and through any other block that names him.

### Metadata threading

- Every block carries the same `source_id: bh268-2026-04-15` and `generated_at: 2026-05-02T00:00:00-10:00` from upstream — a parser can group all blocks for this source by exact-match on source_id.
- Confidence and vault_match_hint propagate from the upstream typed file's `Type confidence` and `Vault match hint` fields.

### Varys whisper format

- Title is 4 words: "Policy subgroup is dying."
- Lede is 9 words.
- Why it matters is 19 words.
- Body is well under 80 words.
- `whisper_category: orphan` — the "policy subgroup is thinning" framing was buried inside what looked like a casual offhand comment about Kieran disappearing. Anansi never picked it up because it didn't crystallize as an entity.
- Edges point at three other blocks in the same atomized file (organization:hawaii-broadband-office, person:burt-lum, note:hsac-convening-2026) — everything resolves.

### Concept handling

- `#use-and-adoption` does NOT get its own block. It threads into the closing `<!-- concepts: ... -->` line along with every other concept from `resources-toc.md` `## Concepts`. The parent source-note will pick it up at ingest time as part of the source's concept tag set.

---

## What you'd see if PA were populated

For comparison — if `projects-areas-toc.md` had a Project at `1.1`, the output would emit:

- No `### 0.0 PA Audit` block
- A `### 1.1 {Project Name} [project]` block following `entity-project.md`
- Discussion blocks could carry `under_project: project:{slug}` edges resolving to that 1.1 block
- The `## Notes` block from `projects-areas-typed.md` would still surface "borderline candidates" as Reviewer-note flags inside the closest related block, not as a standalone audit

That's the path concon takes (next document over). See `references/cross-validation-concon.md` for the cross-check.

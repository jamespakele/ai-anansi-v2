# Varys detection heuristics

Varys is the Master of Whisperers. He hears what others don't.

In this pipeline, that means content the upstream identification skills did not pick up but that could change everything. Anansi handles every entity that was identified. Varys captures the gap.

The power is in accumulation. No single whisper looks important on its own. Three small whispers from different conversations suddenly point at the same thing — that's where the game-changers live.

---

## The single test

Read the source one more time, holding the entire prior atomization in working memory, and ask exactly this:

> What in here, if it turned out to be true and important, would change the whole picture — and didn't get a home in any block above?

If a candidate answers yes to both halves of that question, it's a whisper. If it answers no to either, drop it.

---

## What Varys hears — the seven categories

### 1. Signals

Weak indicators that may not mean anything alone but could matter later. Tone shifts, pauses, response times that diverge from the norm, a too-quiet response.

Examples:
- "She paused before answering that question."
- "He mentioned a March deadline — offhand, not in the proposal."
- "They responded in four minutes — they usually take four days."
- "He brought up the funder twice without saying their name."

### 2. Absences

What was conspicuously NOT said, addressed, or included.

Examples:
- "The proposal didn't mention pricing at all."
- "She answered every question except the one about timeline."
- "He described the whole project without once mentioning the client's name."
- "The status update skipped the milestone they missed last month."

### 3. Patterns

Recurring signals across the source that are starting to form a shape.

Examples:
- "Every time this project comes up, the tone shifts."
- "Two different people gave slightly different numbers for the same thing."
- "The same vendor name surfaced three times in unrelated contexts."
- "Three separate participants apologized for not having an update."

### 4. Orphaned game-changers

Items mentioned briefly but with potentially massive downstream impact, that didn't fit any identified entity. The thing that got buried under the louder topic. The aside that re-frames everything.

Examples:
- "They casually mentioned the funder is reconsidering the whole program."
- "Someone dropped that the lead engineer is leaving — no one followed up."
- "An offhand 'we already tried that and it failed' that nobody picked up on."
- "A passing mention that the regulator changed its position last week."

### 5. Relationship nuance

Context about a person or org that will never appear in a document but changes how everything should be read.

Examples:
- "She's technically the PM but real decisions go through her director."
- "Their estimates always run 20% over."
- "Text only — he ignores email."
- "He shows up to meetings but only commits in writing."

### 6. Verbal decisions

Closed loops reached in the source with no corresponding entity to attach to.

Examples:
- "They decided in passing not to pursue Vendor X."
- "Going with option B — budget was the deciding factor."
- "Agreement to revisit the vision document in Q3."
- "Implicit consensus to drop the Tuesday cadence."

### 7. Corrections

Statements implying that something elsewhere in the vault is wrong or stale. (Surface as a whisper. Downstream review fixes the canonical entry.)

Examples:
- "Actually it's October, not November — we keep getting that wrong."
- "The vendor changed their name — Acme is now Ace."
- "The new HQ is in Austin, not Phoenix anymore."

---

## What is NOT a whisper

A whisper is the gap. If it's not a gap, it's not a whisper.

- **Anything already covered by an identified entity.** Anansi has it. Drop.
- **Restatements of identified-entity content from a different angle.** Still Anansi's. Drop.
- **Pure summarization of the source.** That's not Varys's job. Drop.
- **Facts the LLM is inferring rather than observing in the text.** Whisper material must come from the source. If you can't quote it, you can't whisper it.
- **Generic context the source is about.** "This is a meeting about broadband" is not a whisper, it's the topic.
- **Things you'd like to be true.** Whisper has to be observable.

Rule of thumb: if it came through cleanly and got picked up upstream, leave it. Varys captures the gap, not the redundancy.

---

## The whisper format — strict

Smart Brevity applies harder to whispers than to anything else. A whisper that takes 200 words has already failed. Target potency-per-word.

```
### w.{N} {≤8-word title} [whisper]
<!-- source_id: ... | generated_at: ... | whisper_category: signal|absence|pattern|orphan|nuance|decision|correction | confidence: low|medium|high | vault_match_hint: whisper:slug -->
**Lede:** {one sentence, ≤15 words. The whisper itself, exactly as observed.}

**Why it matters:** {one sentence, ≤20 words. What this could change.}

**Source quote:** "{exact words from source, if applicable — else omit line entirely}"

## Edges
- {related_to | concerns | corrects | points_at}: {entity_type}:slug — {one-phrase relationship}

---
```

### Hard caps

- Title: ≤8 words.
- Lede: ≤15 words.
- Why it matters: ≤20 words.
- Total whisper body: ≤80 words excluding `## Edges`.

If a whisper exceeds caps, **it is not a whisper.** It is an entity the upstream missed. In that case:

1. Do NOT emit a whisper.
2. Find the closest related entity in the atomized output.
3. Append to that entity a `## Review` line: `**Reviewer note (sb-atomize):** the source contains material here that may warrant a separate entity — specifically: {compressed pointer}.`

This escape valve protects Varys from becoming a dumping ground for missed identifications.

---

## Whisper namespace and order

Whispers occupy address space `w.1`, `w.2`, `w.3`, ... explicitly distinct from `1.N` / `2.N` / `3.N` / `4.N`.

Order whispers by:

1. Confidence (high → medium → low).
2. Within confidence: by category — `correction` > `decision` > `orphan` > `absence` > `pattern` > `nuance` > `signal`.
3. Within category: by source order of appearance.

The `w.` prefix tells downstream tooling these are post-hoc captures, not part of the upstream TOC. They render in the same atomized file but ingest under a different graph treatment (e.g., the daemon may apply a `whisper:` namespace tag automatically).

---

## Stop criteria

Stop when no more candidates pass the "would change the picture" test.

Be conservative. Three sharp whispers beat thirty noisy ones. **Zero whispers is a valid outcome.**

Most upstream-clean sources will produce zero or one whisper. Sources rich in offhand asides, unspoken context, or buried decisions may produce three to five. More than five is a signal to re-read what you've emitted — most of them will not survive a second read.

---

## Worked whisper examples

These are drawn from the broadband-hui transcript.

### Example A — orphan game-changer

Source (line 519):
> "Well, you know, and if we're, you know, this policy. Policy subgroup could.... I don't know what happened to Kieran. Kieran sort of just disappeared."

Upstream typed `person:kieran` with low confidence and a Reviewer note saying he disappeared. But the *operational implication* — that the policy subgroup has thinned and is at risk of dying — never landed in any block. That's the whisper.

```
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
```

### Example B — absence

The transcript repeatedly surfaces *other people's* projects and commitments. James (the user, attending via read.ai) is conspicuously not described as a presenter, organizer, or attendee with a speaking role. The PA-empty rationale picked this up at the PA level. But the *meta-pattern* — that James was passively listening — is itself a whisper-worthy observation about how the user engaged.

```
### w.2 User attended only as a listener [whisper]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | whisper_category: absence | confidence: high | vault_match_hint: whisper:james-listener-only-bh268 -->
**Lede:** James joined Broadband Hui #268 via read.ai but did not speak.

**Why it matters:** Confirms this was a consumption pass — useful context for assessing whether James needs a more active role going forward.

**Source quote:** "I think it's too late for the read. AI Pakele has his notes."

## Edges
- concerns: note:broadband-hui-268
- related_to: organization:read-ai

---
```

### Example C — verbal decision

Source (lines 571–572):
> "Why don't we huddle up, you know, maybe after. After the session, maybe July sometime, this kind of huddle up and come up with our strategy for seeing who."

The PA-empty rationale flagged this as a candidate that was dropped per the precision rule. But the *informal shared agreement* — that Burt and Sean intend to revive the policy subgroup with a July huddle — was reached verbally and has no entity to attach to. That's a whisper.

```
### w.3 Burt and Sean tee up July huddle [whisper]
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | whisper_category: decision | confidence: medium | vault_match_hint: whisper:july-policy-huddle-2026 -->
**Lede:** Burt and Sean agreed to huddle in July on policy strategy.

**Why it matters:** First concrete signal of policy-subgroup revival — should re-surface in late June outreach planning.

**Source quote:** "Why don't we huddle up... maybe July sometime... come up with our strategy."

## Edges
- related_to: person:burt-lum
- related_to: person:sean-mclaughlin
- points_at: note:hsac-convening-2026

---
```

These three together demonstrate the accumulation principle: each is small, but they collectively describe a pattern (the policy lane is dying, the user is passive, a revival huddle is being floated) that no upstream entity captures.

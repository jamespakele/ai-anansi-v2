# sb-atomize — design notes

This is the rationale doc for the skill's structural decisions. Read it once. Don't re-read on every invocation.

---

## (a) Sequential vs. parallel processing

**Decision: Sequential, in this order.** PA → Resources (people, orgs, notes) → Discussion → Concepts → Varys.

**Why not parallel:**

Discussion blocks (Section 3) carry `## Edges` that point to entity slugs from Sections 1, 2, and 4. If Discussion is atomized before its dependencies, edges become forward references and the LLM may invent slugs that don't end up matching what gets emitted.

Sequential flow lets every block reference only entities that have already been written above it. This means at any point during the run, every emitted edge slug is verifiable against the output buffer. That property is load-bearing for the downstream parser — `anansi_ingest_atomized` resolves edges by exact slug match, and a typo silently drops the relationship.

**Why this order specifically:**

- **PA first** because Projects and Areas are organizing structures — Discussion blocks can use `under_project: project:slug` and `under_area: area:slug` edges if PA exists.
- **Resources second** because Discussion blocks reference person/org/note slugs as participants, organizations, and references.
- **Discussion last** (of the entity passes) because it's the heaviest content and benefits from having both PA and Resources atomized in the LLM's working memory.
- **Concepts threaded into a single closing line** rather than emitted as blocks. Concepts are shapeless tags, not first-class entities.
- **Varys at the very end**, with the entire prior atomization in context.

**Edge case — empty PA:**

Skip Sections 1 + 2 entirely. Emit one `### 0.0 PA Audit [note]` block carrying the rationale from `projects-areas-typed.md` `## Notes`. The audit is itself a `note` so the trail stays on-graph. Verified against `output/broadband-hui/` where PA is `_(none found)_` with a 7-bullet rationale block.

---

## (b) Varys pass timing

**Decision: Once at the very end, across the whole source.**

**Why not per-entity-stream:**

Per-entity Varys runs are tempting because they keep context small, but they create three problems:

1. **Duplicate whispers** — the same offhand comment shows up as a person-context whisper, then again as a discussion-context whisper, then again as a concept-context whisper.
2. **No "what's missing" view** — Varys's job is to capture what didn't fit anywhere. You can only know that after seeing what fit.
3. **Premature commitment** — a comment that looks whisper-worthy mid-pass may turn out to belong cleanly inside a discussion block atomized later.

End-of-pipeline Varys has the full atomization in context. The single question — "what in the source did NOT make it into any block above?" — is answerable only there.

**Stop criteria:**

Three sharp whispers beat thirty noisy ones. Zero is valid. Stop when no more candidates pass the "would change the picture" test.

---

## (c) Output sharding format

**Decision: One combined markdown file with `---` delimiters.**

**Why not one file per entity:**

- The downstream consumer is `anansi_ingest_atomized`, which already accepts a single `---`-delimited document and shards internally. Producing N files would force the caller to re-bundle.
- The opening `<!-- anansi-atomize: ... -->` comment header and the closing `<!-- concepts: ... -->` line are document-level markers that don't have a natural home in per-entity files.
- A single file keeps the source_id / generated_at threading visible at a glance during debugging.
- `awk '/^---$/'` shards to per-entity files in one line if a downstream caller wants that.

**Block-level metadata:**

Per-block YAML frontmatter would conflict with the `---` split token. Instead, every block has an HTML comment line directly under the heading carrying source_id, generated_at, confidence, and vault_match_hint. That comment is parser-stable and human-readable.

---

## (d) Empty-PA handling

**Decision: Skip Section 1 + 2 entirely. Emit a single audit note at address `0.0`.**

**Why not silently drop:**

The rationale block in `projects-areas-typed.md` `## Notes` is high-signal content — it explains *why* nothing was extracted, names the projects and areas that were considered and dropped, and identifies "borderline candidates" to flag for future ingests. Throwing it away costs auditability.

**Why not promote any of the considered candidates:**

`para-projects-areas` is fail-closed by design. If it dropped a candidate per the precision rule, this skill must respect that decision. Promoting on the strength of "the rationale mentioned it" would defeat the upstream skill's whole purpose.

**Audit block shape:**

```
### 0.0 PA Audit [note]
<!-- source_id: ... | generated_at: ... | confidence: high | vault_match_hint: note:source-id-pa-audit -->
{Lede: one sentence summary of why PA is empty, drawn from the rationale}.

**Why it matters:** {one sentence on what this signals about the source — typically "consumption-shaped" or "other people's commitments dominate"}.

## Considered and dropped
- {Compressed bullet for each candidate the rationale named, with reason}

## Borderline candidates
- {Compressed bullet for each "would extract if X" candidate}

---
```

This keeps the trail on-graph while honoring "PA empty is a successful run."

---

## (e) Reviewer notes and low-confidence surfacing

**Decision: Reviewer notes get a `## Review` section at the bottom of the block. Low-confidence + reviewer note → also flag at the top.**

**Three signals from upstream that must survive into the atomized output:**

1. **`Type confidence`** (low / medium / high) — carry into the metadata HTML comment as `confidence: ...`. Downstream filtering can use this to surface low-confidence entities for review.
2. **`_Reviewer note:_`** content — copy verbatim into a `## Review` section, prefixed with `**Reviewer note:**`. Don't editorialize. Don't compress. The reviewer note is itself the reviewer's voice.
3. **Low confidence + reviewer note together** — the strongest review signal. Add `<!-- review-needed: low confidence -->` as a comment line at the top of the block. Tooling can grep for it.

**Why surface, not silently drop:**

Reviewer notes are exactly the breadcrumbs that prevent silent error accumulation. Anansi's read tools (`anansi_search`, `anansi_filter`) can pull all blocks with `review-needed` flags for a weekly review pass. Burying them in upstream files breaks the chain.

---

## (f) Varys pass timing and stop criteria

Detailed already in (b). Two additions:

**Whisper namespace:**

Whispers occupy address space `w.1`, `w.2`, ... — explicitly distinct from `1.N`, `2.N`, `3.N`, `4.N`. The `w.` prefix signals to downstream tooling that these are post-hoc captures, not part of the upstream TOC.

**The whisper-vs-entity escape valve:**

If a whisper exceeds the hard caps (lede ≤15 words, why-it-matters ≤20 words, body ≤80 words), it's actually an entity the upstream missed. In that case:

1. Do NOT emit a whisper.
2. Find the closest related entity in the atomized output.
3. Append a `## Review` line to that entity: `**Reviewer note (sb-atomize):** the source contains material here that may warrant a separate entity — specifically: {compressed pointer}.`
4. The next ingest cycle, with the user's review, can promote it to a real entity.

This prevents Varys from becoming a dumping ground for missed identifications.

---

## What this skill explicitly does NOT do

- **Does not invent entities.** If the upstream typed pipeline didn't identify something, this skill doesn't promote it. Whisper or Reviewer-note it.
- **Does not summarize the source.** This is an atomization skill. Summary belongs to `secretary` or `smart-brevity` standard mode.
- **Does not re-extract or re-type.** Stages 1 and 2 own that. Trust their outputs.
- **Does not write to anansi.** That's `anansi_ingest_atomized`'s job, called downstream.
- **Does not make routing decisions outside the entity-routing rules.** When in doubt, see `entity-routing-rules.md` and pick the rule. If no rule fits, surface it as a Reviewer note rather than guessing.

---

## Why slow is smooth

Two failure modes account for almost all bad atomization output:

1. **Boundary leakage** — narrative content lands in person blocks, person facts land in discussion blocks, projects edges to slugs that don't exist.
2. **Whisper-as-entity drift** — the LLM tries to make a whisper hold the weight of a missed entity, and the whisper bloats past its caps.

Both are pace failures. Reading all five inputs first (Step 1), then atomizing in stable order (Steps 2–8), then running the Varys pass last with full context (Step 9) — that's the correct sequence. Skipping any of it produces noise.

The Smart Brevity discipline is itself a slowness discipline: every cut word is a deliberate decision, not a reflex. This skill applies that posture to atomization.

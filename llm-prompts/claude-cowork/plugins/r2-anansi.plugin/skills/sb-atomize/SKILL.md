---
name: sb-atomize
description: >
  Stage 3 of the PARA atomization pipeline. Takes a source content file plus
  the four pipeline outputs from para-projects-areas and para-resource-entities
  (projects-areas-toc.md, projects-areas-typed.md, resources-toc.md,
  resources-typed.md) and emits one Smart Brevity-compliant atomic note per
  identified entity, plus zero or more Varys whisper notes for buried signals.
  Each note conforms to its matching template in references/templates/. Output
  is one combined markdown file with --- delimited blocks, ready for ingestion
  via anansi_ingest_atomized. Empty-PA runs are first-class. The Varys pass
  runs last and is conservative — zero whispers is valid. Triggers: "atomize
  the typed pipeline output", "smart brevity atomize this", "/sb-atomize",
  "/sb-atomize", "compress the typed entities", "run stage 3 on
  this source", "atomize the entities and run varys", "produce per-entity
  smart-brevity notes from this", "varys pass on this transcript".
argument-hint: "[source content file path] [projects-areas-toc.md] [projects-areas-typed.md] [resources-toc.md] [resources-typed.md]"
---

# sb-atomize

Stage 3 of the PARA atomization pipeline. Stages 1–2 (`para-projects-areas` and `para-resource-entities`) identify and scope. This skill distills content per identified entity AND runs a final Varys whisper pass over the source to surface buried signal that did not fit any identified entity.

The output is **one markdown file** containing one Smart Brevity-compliant block per identified entity, separated by `---`, plus zero-or-more Varys whisper blocks at the end. Each block conforms to its matching template in `references/templates/`. The output is trivially shardable into atomic notes (one entity = one block) and can be fed directly to `anansi_ingest_atomized`.

> "Brevity is confidence. Length is fear." — and the gap is where Varys lives.

---

## When to invoke

Trigger this skill when the user:

- Hands you a source content file plus the four typed pipeline outputs and asks to atomize per identified entity
- Says "atomize the typed pipeline output", "/sb-atomize", "/sb-atomize", "smart brevity atomize this", "compress the typed entities", "run stage 3 on this source", "atomize the entities and run varys", "produce per-entity smart-brevity notes from this", "varys pass on this transcript"
- Is at Stage 3 of the atomization pipeline: `para-projects-areas` + `para-resource-entities` → **sb-atomize** → `anansi_ingest_atomized`

Do not invoke for:

- A single document compression with no upstream typed pipeline (use `smart-brevity`)
- Identifying or typing entities (those are Stages 1–2)
- The full end-to-end pipeline starting from raw text (use `atomize`)
- A standalone whisper-only pass without the upstream typed outputs

---

## Inputs

This skill consumes **five files** in this exact shape (verified against `output/broadband-hui/` and `output/concon/`):

1. **Source content file** — the raw transcript, email thread, article, book chapter, or prose. Plain `.txt` or `.md`.
2. **`projects-areas-toc.md`** — frontmatter (source_id, generated_at, skill, parallel_pair) + `## 1. Projects` + `## 2. Areas`. Each section may be `_(none found)_` (empty-PA case).
3. **`projects-areas-typed.md`** — frontmatter (same source_id) + `## Projects` + `## Areas` + `## Notes`. The Notes block carries the rationale when PA is empty.
4. **`resources-toc.md`** — frontmatter (same source_id) + `## 3. Discussion` (decimal-addressed, each tagged `[discussion]`) + `## 4. Resources` (decimal-addressed, each tagged `[person:slug]` / `[organization:slug]` / `[note:slug]`) + `## Concepts` (hashtags with one-line definitions).
5. **`resources-typed.md`** — frontmatter (same source_id) + `## Resources` + one `### type:slug` block per entity with **Name, Template, Type confidence, Identity fields, Evidence, Concepts, Vault match hint**, and optional `_Reviewer note:_`.

If any of the five is missing, stop and tell the user what is missing rather than improvising. The four pipeline files share a `source_id` that **MUST** be threaded into every emitted block.

### File location is the user's call

This skill does not impose a file layout. It reads from the paths it is given and writes the atomized output alongside them.

- **Sandbox-by-default.** If invoked in a fresh sandbox session with no folder context, the five inputs and the atomized output all live in the working directory and disappear at session end. That is fine for one-shot runs.
- **Persist-by-folder.** If the user wants to keep the run, they invoke from inside a project folder (or hand in paths inside one). Convention is `output/<source-slug>/` containing `transcript.txt` (or equivalent), the four pipeline files, and `<source-slug>-atomized.md` — but that's a user choice, not a skill rule.

Do not move, copy, or relocate input files. Do not assume an `output/` directory exists. Write the atomized output to the same directory as the four pipeline files unless the user explicitly says otherwise.

---

## Output

**Two files** are emitted for every run:

### 1. `{source-slug}-atomized.md`

The full atomic note set, with this shape:

```
<!-- anansi-atomize: {source title} | {N} blocks | {date} | source_id: {source_id} -->

[optional ## 0 audit block — emitted only when PA is empty, capturing the rationale]

--- 

[Section 1 — Projects, in TOC order, one block per project with address 1.N]

--- 

[Section 2 — Areas, in TOC order, one block per area with address 2.N]

--- 

[Section 3 — Discussion, in TOC order, one block per topic_discussion with address 3.N]

--- 

[Section 4 — Resources, in TOC order, one block per person/organization/note with address 4.N]

--- 

[Whisper section — zero or more, addresses w.1 w.2 ..., emitted only after every entity above is atomized]

<!-- concepts: #tag1 #tag2 #tag3 ... -->
```

Empty sections are omitted entirely (no heading, no `_(none found)_` placeholder — the parser splits on `---` and empty sections create empty blocks).

Each block ends with `---` on its own line. The closing `<!-- concepts: ... -->` line carries Section 5 concepts (no per-block emission for hashtag concepts — they thread into the source's note frontmatter).

### 2. `{source-slug}-toc.md`

A single combined TOC — a manifest of every entity in the atomized file, assembled from `projects-areas-toc.md` and `resources-toc.md`. Shape:

```
---
source_id: {source_id}
generated_at: {generated_at}
source_title: {source title}
total_blocks: {N}
skill: sb-atomize
---

## 1. Projects
- 1.1 [project] {Name}
- 1.2 [project] {Name}
  - 1.2.1 [project] {Name}

## 2. Areas
- 2.1 [area] {Name}

## 3. Discussion
- 3.1 [discussion] {short title}
- 3.2 [discussion] {short title}

## 4. Resources
- 4.1 [person:{slug}] {Name}
- 4.2 [organization:{slug}] {Name}
- 4.3 [note:{slug}] {Name}

## Whispers
- w.1 [whisper] {title}

## Concepts
- `#tag1` — {definition}
- `#tag2` — {definition}
```

Empty sections are omitted. Whispers section is omitted if no whispers were emitted. Concepts are carried verbatim from `resources-toc.md`. This file is sent to the MCP alongside the atomized file as the ingest manifest — the parser can use it to validate block count and resolve slugs without parsing the full content file.

Both files are written to the same directory as the four input pipeline files.

---

## Process

Slow is smooth, smooth is fast. Do every step in order. Do not skip the reading.

### Step 1 — Read all five inputs end-to-end before writing anything

Read in this order:

1. `projects-areas-toc.md` — to see the PA shape (empty? populated? sub-projects?)
2. `projects-areas-typed.md` — to capture entity fields and the `## Notes` rationale block
3. `resources-toc.md` — to see Discussion shards, Resource list, Concepts
4. `resources-typed.md` — to capture per-entity Templates, Type confidence, Reviewer notes
5. The source content file — last, after you know what to look for

This order is deliberate: by the time you read the source, you already know which entities are in scope. That makes the source read a **content-mining pass**, not an identification pass.

### Step 2 — Capture metadata for threading

Pull from any of the four pipeline files (they agree):

- `source_id`
- `generated_at`
- `parallel_pair` (informational)

Every emitted block carries these in an HTML comment immediately under the heading. Example:

```
<!-- source_id: bh268-2026-04-15 | generated_at: 2026-05-02T00:00:00-10:00 | confidence: high | vault_match_hint: person:jaren-dhhl -->
```

### Step 3 — Decide the source's input type

This drives the Discussion shard template. Read the first 50 lines of the source and pick the closest match from the Smart Brevity input vocabulary:

| Source signal | Type | Discussion template |
|---|---|---|
| Speaker tags + timestamps + meeting host | `meeting` | `meeting-topic-discussion.md` |
| Subject:, From:, To:, multiple stacked emails | `email_thread` | `email-exchange.md` |
| Numbered chapters or "Ch. N" / book metadata | `book` | `book-chapter.md` |
| Newsletter masthead + ranked items | `newsletter` | `newsletter-item.md` |
| Slide labels, deck shape | `presentation` | `presentation-slide.md` |
| YouTube chapter timestamps | `youtube_video` | `youtube-chapter.md` |
| Anything else | `generic_prose` | `meeting-topic-discussion.md` (closest content_unit shape) |

Load the matching template from `references/templates/`. Use its `identity_fields` and body shape as the contract for Section 3 blocks.

### Step 4 — Handle the empty-PA case (decision point)

Read `projects-areas-toc.md` Section 1 and Section 2.

**If both are `_(none found)_`:**

1. Emit a single `### 0.0 PA Audit [note]` block at the top of the output. Body: copy the rationale from `projects-areas-typed.md` `## Notes` block, compressed via Smart Brevity rules (lede + why-it-matters + bullets). This preserves the auditable trail on-graph.
2. Skip Section 1 (Projects) and Section 2 (Areas) entirely. Do **not** invent projects or areas from the source.
3. Proceed to Section 3.

**If either is populated:** emit Section 1 and/or 2 blocks per `## Section 1 — Projects` and `## Section 2 — Areas` rules below. The PA audit block is omitted.

The `## Notes` block in `projects-areas-typed.md` may also list "borderline calls" or "candidates dropped per the precision rule" even when PA is non-empty. **Surface these as comment-line flags inside the closest related block** — do not emit them as standalone notes.

### Step 5 — Section 1: atomize Projects (when present)

For each `### project:slug` block in `projects-areas-typed.md`:

1. Load `entity-project.md` template. The body shape: `## Lede` → `## Why` → `## Content` (containing `### Identity`, `### Summary`, `### Narrative`) → `## Edges`.
2. Map fields: `Name → name`, `Goal → goal`, `End date → end_date`, `Evidence → ### Narrative paragraph`, `Parent area → ## Edges: under_area: area:slug`, `Parent project → ## Edges: sub_project_of: project:slug`.
3. Block heading: `### {address} {Name} [project]` (e.g., `### 1.2 HIPA ConCon Documentary Film [project]`).
4. Apply Smart Brevity rules: `## Lede` (≤25 words, standalone outcome statement). `## Why` (1–2 sentences on stakes or deadline — does NOT repeat the lede).
5. `## Content` is the outer block. Inside it: `### Identity` (Goal, Status, End Date), `### Summary` (one source-agnostic paragraph), `### Narrative` (2–4 paragraphs of substantive prose from the source).
6. `## Edges` carries all relational fields: `contributed_by: person:slug — role`, `under_area: area:slug`, `sub_project_of: project:slug`, `stakeholder: person:slug`, etc.
7. Carry `Type confidence` (or `Confidence` field, whichever the typed file uses) into the block's metadata comment.
8. Carry `_Reviewer note:_` content into a `## Review` section at the bottom of the block.
9. End with `---` on its own line.

### Step 6 — Section 2: atomize Areas (when present)

Same procedure as Section 1, but using `entity-area.md` template. Body: `## Lede` → `## Why` → `## Content` (containing `### Identity` with Standard/Owner/Description, `### Summary`, `### Narrative`) → `## Edges` (with `stakeholder: person:slug — role` entries).

The lede for an Area is one sentence naming the territory and the standard. `## Why` covers why this responsibility exists or what it serves.

### Step 7 — Section 3: atomize Discussion shards

For each entry in `resources-toc.md` `## 3. Discussion`:

1. Find the corresponding span in the source content file (use the descriptive text after the bracket tag as a search hint).
2. Load `meeting-topic-discussion.md` (or matching content_unit per Step 3).
3. Block heading: `### {address} {short title from TOC} [discussion]` (e.g., `### 3.2 DHHL Use & Adoption RFP update from Jaren [discussion]`).
4. `## Lede` (≤25 words): one sentence naming the topic and what was discussed.
5. `## Why` (1–2 sentences): adds perspective or downstream consequence. Does not repeat the lede.
6. `## Content` is the outer block. Inside it:
   - `### Narrative` — 3–6 sentences. What was said, not who said it. No verbatim quotes. Smart Brevity tone.
   - `### Decisions` — bullet list of concrete outcomes. If none: `- [no decision reached]`.
   - `### Action Items` — bullet list shaped `Owner to do X by date` where information is available.
7. `## Edges` carries ALL relational fields — participants, organizations, and cross-entity links — as typed edge entries. Participants and organizations are NOT separate sections; they all live here:
   - `participated_by: person:slug` — people who spoke in this topic (only those who participated, not all attendees)
   - `mentioned_org: organization:slug` — orgs mentioned or represented
   - `presented_by: person:slug` — who drove the topic
   - `references: note:slug` — named documents/programs/events referenced
   - `under_project: project:slug` — if the discussion advances a project from Section 1
   - `under_area: area:slug` — same for areas
   - `related_to: person:slug` — persons relevant but not present
8. End with `---`.

**Boundary discipline:** The discussion block holds discussion content only. Person identity facts go in Section 4 person blocks. Org identity facts go in Section 4 organization blocks. If a piece of information could go in either, ask: *what kind of fact is it?* (identity → resource block; participation/mention → discussion block `## Edges`).

### Step 8 — Section 4: atomize Resources

For each `### type:slug` block in `resources-typed.md`, walk in TOC order from `resources-toc.md` `## 4. Resources`. The TOC's decimal address (4.N) is the block's stable address.

Choose block shape by the type tag:

#### 4.N person blocks

Load `entity-person.md`. Strict identity-only:

```
### {address} {Name} [person]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: person:slug -->

## Lede
{Name} is {single identifying noun phrase from Identity fields}.

## Content

### Identity
- Met via: {met_via field, ≤15 words, Smart Brevity style}

## Review
**Reviewer note:** {if present in typed file, else omit entire ## Review section}

---
```

**Strict prohibitions on person blocks:**

- No `## Edges` section. Persons are not convergence points. Relationships surface through the blocks of the things the person is connected to.
- No `## Contact` section unless contact info (email/phone) is explicitly stated in the source.
- No narrative about their involvement, role in the meeting, or what they said. That belongs in Section 3 discussion blocks.
- `## Lede` is identity-only — one noun phrase. "Cameron Hurt is the new state director of Common Cause Hawaii." Not "Cameron Hurt is the new state director who committed to making ConCon a signature focus."
- No `## Why` on person blocks.

#### 4.N organization blocks

Load `entity-organization.md`. Lede + Content (Identity + Context) + Edges (affiliated persons only):

```
### {address} {Name} [organization]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: organization:slug -->

## Lede
{Org name} is a {type} {brief descriptor}.

## Content

### Identity
- Full name: {full_name — only if different from name}
- Type: {type}
- Domain: {domain}

### Context
- {source-bound canonicalization fact or stable identity fact 1}
- {fact 2}

## Edges
- affiliated_with: person:slug
- affiliated_with: person:slug

## Review
**Reviewer note:** {if present, else omit}

---
```

`### Context` (inside `## Content`) accumulates source-bound canonicalization facts and stable identity facts — name variants resolved, founding history, key leadership. It does NOT contain project-specific activity or meeting-specific behavior. Those belong in discussion blocks (Section 3) or project blocks (Section 1). Project/vendor relationships do NOT go here.

#### 4.N note blocks

Load `entity-note.md`. Lede + optional Why + Content (Summary + Narrative) + Edges:

```
### {address} {Name} [note]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: note:slug -->

## Lede
{One sentence — the most important fact about what this is, ≤25 words.}

## Why
{One sentence on relevance — omit entire ## Why block if self-evident}

## Content

### Summary
{One sentence.}

### Narrative
{2–4 sentences of source-agnostic description. Smart Brevity tone.}

## Edges
- {relationship_verb}: {entity_type}:slug
- {relationship_verb}: {entity_type}:slug

## Review
**Reviewer note:** {if present, else omit}

---
```

Common note edge verbs: `presented_by`, `produced_by`, `under_project`, `references`, `referenced_by`, `managed_by`, `concerns`. Always match slugs to the exact slug in the entity's own block heading.

### Step 9 — Run the Varys pass (LAST — only after every above section is written)

This is the final pass and the one most LLMs will be tempted to skip. Don't.

#### Pre-conditions

- Every entity in `resources-typed.md` and `projects-areas-typed.md` has been atomized.
- The full atomization above is in your context window.

#### The single question

Read the source content file one more time, asking exactly this:

> What in here, if it turned out to be true and important, would change the whole picture — and didn't get a home in any block above?

#### What counts as a whisper

See `references/varys-detection-heuristics.md` for the full taxonomy. Seven categories:

1. **Signals** — weak indicators (a pause, a side-comment, a too-fast response, a too-slow response).
2. **Absences** — what was conspicuously NOT said when it should have been.
3. **Patterns** — recurring shapes across the source forming an emergent picture.
4. **Orphaned game-changers** — items mentioned briefly but with potentially massive downstream impact.
5. **Relationship nuance** — how a person or org should be read (text-only, decisions actually made by their director, estimates run 20% over, etc.).
6. **Verbal decisions** — closed loops in the source with no entity to attach to.
7. **Corrections** — statements implying something elsewhere in the vault is wrong or stale.

#### What is NOT a whisper

- Anything already in an atomized block above (Anansi has it).
- Restated identified-entity content from a different angle.
- Pure summarization of the source.
- Facts the LLM is inferring rather than observing in the text.

#### Whisper format (hard caps)

```
### w.{N} {≤8-word title} [whisper]
<!-- source_id: ... | generated_at: ... | whisper_category: signal|absence|pattern|orphan|nuance|decision|correction | confidence: low|medium|high | vault_match_hint: whisper:slug -->

## Lede
{One sentence, ≤15 words. The whisper itself, exactly as observed.}

## Why
{One sentence, ≤20 words. What this could change.}

**Source quote:** "{exact words from source, if applicable — else omit line entirely}"

## Edges
- {related_to | concerns | corrects}: {entity_type}:slug — {one-phrase relationship}

---
```

Hard caps: `## Lede` ≤15 words, `## Why` ≤20 words, total whisper body ≤80 words excluding edges. No `## Content` block on whispers. Whisper exceeds caps → it's not a whisper, it's an entity the upstream missed. Flag it as a Reviewer note on the closest related entity, do **not** emit it.

#### Stop criteria

Stop when no more candidates pass the "would change the picture" test. Be conservative — three sharp whispers beat thirty noisy ones. **Zero whispers is a valid outcome.**

### Step 10 — Emit the closing concepts line

From `resources-toc.md` `## Concepts`, take every hashtag and concatenate them as the closing line:

```
<!-- concepts: #tag1 #tag2 #tag3 ... -->
```

No standalone blocks for concepts. They thread into the parent source's frontmatter at ingest time.

### Step 10.5 — Write the combined TOC file

After the atomized file is complete, write `{source-slug}-toc.md` to the same directory. This is a straight assembly from already-read inputs — no new source reading required.

1. Frontmatter: pull `source_id`, `generated_at` from the pipeline files. Set `source_title` from the `<!-- anansi-atomize: ... -->` header. Set `total_blocks` to the block count from that same header.
2. `## 1. Projects` — copy directly from `projects-areas-toc.md` Section 1 (decimal addresses + `[project]` tags + names, preserving sub-project nesting). Omit if empty.
3. `## 2. Areas` — copy from `projects-areas-toc.md` Section 2. Omit if empty.
4. `## 3. Discussion` — copy from `resources-toc.md` `## 3. Discussion` (addresses + `[discussion]` tags + short titles). Omit if empty.
5. `## 4. Resources` — copy from `resources-toc.md` `## 4. Resources` (addresses + typed bracket tags + names). Omit if empty.
6. `## Whispers` — list each whisper emitted in Step 9 (`w.N [whisper] {title}`). Omit section entirely if no whispers were emitted.
7. `## Concepts` — copy the full hashtag + definition list from `resources-toc.md` `## Concepts` verbatim.

The combined TOC is a manifest, not a summary. Do not paraphrase or compress — copy addresses, type tags, and names exactly as they appear in the upstream files so the parser can use them as a lookup table against the atomized blocks.

### Step 11 — Self-check

- [ ] Every decimal TOC entry has exactly one block?
- [ ] Every block ends with `---`?
- [ ] Every block has the metadata HTML comment (source_id, generated_at, confidence, vault_match_hint)?
- [ ] Every block uses `## Lede` as a labeled section (not an unlabeled opening paragraph)?
- [ ] `## Why` is a labeled section (not `**Why it matters:**` inline bold)?
- [ ] `## Content` is present on all project/area/discussion/note blocks, with sub-sections nested as `### Identity`, `### Summary`, `### Narrative` (etc.) inside it?
- [ ] Person blocks have NO `## Edges` section and NO narrative about involvement?
- [ ] Person blocks use `## Content > ### Identity` with `- Met via:` bullet?
- [ ] Organization `### Context` contains only source-bound canonicalization facts — NOT project-specific activity?
- [ ] Discussion blocks: participants and mentioned orgs are in `## Edges` as `participated_by:` and `mentioned_org:` — NOT in separate `## Participants` / `## Organizations` sections?
- [ ] Discussion blocks: body prose is in `## Content > ### Narrative` (not a flat `## Content` paragraph)?
- [ ] Whisper blocks: NO `## Content` block — only `## Lede`, `## Why`, optional source quote, `## Edges`?
- [ ] Discussion block edges point to slugs that exist in this same output?
- [ ] PA empty case → audit block at top, no invented projects/areas?
- [ ] Reviewer notes from typed files surfaced as `## Review` sections?
- [ ] Lede sentences standalone (no "as noted in 3.2…")?
- [ ] Smart Brevity rules applied: bold key terms in bullets? No adverbs/qualifiers?
- [ ] Varys pass run AFTER everything else, conservatively?
- [ ] Closing `<!-- concepts: ... -->` line present?
- [ ] Opening `<!-- anansi-atomize: ... -->` comment header present?
- [ ] Combined TOC file (`{source-slug}-toc.md`) written to same directory as atomized file?
- [ ] TOC `total_blocks` count matches the block count in the atomized file header?
- [ ] TOC addresses, type tags, and names match the atomized file exactly (no paraphrasing)?
- [ ] TOC Whispers section omitted if no whispers were emitted?

---

## Design decisions

See `references/design-notes.md` for the full rationale. Summary:

- **Sequential, not parallel.** PA → Resources → Discussion → Concepts → Varys. Sequential lets Discussion edges resolve to entity slugs that already exist in the output.
- **Varys runs last, once.** Per-entity-stream Varys creates duplicates. End-of-pipeline Varys has full context to know what's already covered.
- **One combined file with `---` delimiters.** Matches the canonical `sb-atomized.md` contract and feeds `anansi_ingest_atomized` directly. Trivially shardable downstream.
- **Empty-PA emits one audit note.** Preserves the rationale on-graph without inventing projects/areas.
- **Reviewer notes survive into a `## Review` section.** Low-confidence + reviewer note → `<!-- review-needed: low confidence -->` flag at the top of the block.
- **Whisper hard caps are non-negotiable.** Caps exceeded → it's an entity, not a whisper.

---

## References

- `references/output-format.md` — block templates by entity type with worked examples
- `references/entity-routing-rules.md` — which information goes in which block (boundary discipline)
- `references/varys-detection-heuristics.md` — Varys pass taxonomy with examples
- `references/design-notes.md` — rationale for the design decisions above
- `references/templates/` — vendored entity templates (load by `Template:` field)
- `resources/sb-atomized.md` — canonical output-format contract
- `plugins/r2-anansi.plugin/skills/smart-brevity/SKILL.md` — sibling Smart Brevity skill (single-document mode)

---

## Worked example

A full worked example against `output/broadband-hui/` is in `references/example-broadband-hui.md`. It atomizes:

- Section 3.2 (DHHL Use & Adoption RFP discussion)
- Section 4.8 (person:jaren-dhhl)
- Section 5 concept `#use-and-adoption`
- One Varys whisper drawn from the transcript that did not surface as an upstream entity

Read it after this SKILL.md — it shows template adherence, Smart Brevity rules, edge slugs, metadata threading, and the Varys format under hard caps.

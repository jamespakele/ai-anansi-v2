---
name: para-projects-areas
description: >
  Precision-tuned PA-only extractor. Emits two paired markdown files:
  projects-areas-toc.md (## 1. Projects, ## 2. Areas, decimal addresses,
  [area:2.N] cross-refs, sub-projects nest 1.1.1) and
  projects-areas-typed.md (per-entity fields + Notes), joined by a shared
  source_id. Aligned with para-toc. Pairs with para-resource-entities
  for the Resource side. Scans email, thread, meeting notes, transcript,
  article, prose for the user's active Projects (outcome + deadline +
  commitment) and ongoing Areas (standard + direct responsibility).
  Ignores Resources, Concepts, Archive. Low false-positive rate: when in
  doubt, emits nothing. Optionally accepts known_projects and known_areas
  blocks at the top (triple-dash separator) to mark each existing or new.
  Triggers: "extract projects and areas", "find PA in this", "what
  projects and areas are here", "/para-pa", "PA extract", "identify
  projects and areas", "pull PA from this", "PA toc". Grounded in Forte's
  BASB and PARA Method.
argument-hint: "[document content; optionally prefixed with known_projects/known_areas blocks]"
---

# para-projects-areas

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

A high-precision document scanner. Read an input — an email, thread, meeting summary, transcript, article, or any prose — and emit **two files**: a TOC file (`projects-areas-toc.md`) for navigation and a typed-fields file (`projects-areas-typed.md`) for entity detail. The two files share a `source_id` and `generated_at` so downstream code can rejoin them. Both contain just the user's **Projects** and **Areas** (in that order, matching `para-toc`'s actionability-descending convention). Nothing else. No Resources. No Concepts. No Archive items. The skill's first commitment is to refuse to over-identify: a quiet output is a valid output, and silence beats a false-positive Project or Area every time.

This is the PA-only slice of `para-extract`, paired with `para-resource-entities` for the resource side. Same Forte-grounded decision rules, same input format, same field semantics — but with two structural twists adapted from `para-toc`:

1. **TOC structure** uses decimal addresses (`1.1`, `1.1.1`, `2.1`). Projects come first under `## 1. Projects`, Areas second under `## 2. Areas`.
2. **Sub-projects nest** under their parent project as `1.1.1`, `1.1.2`, etc. — but only when the document explicitly signals the nesting relationship. The precision rule applies: when ambiguous, keep projects flat.
3. **Projects that belong to an identified Area carry a cross-reference tag** of the form `[area:2.1]`, where `2.1` is the area's decimal address in this same TOC. Every member-project links explicitly to its parent area, and the cross-reference must always resolve to a real address in Section 2 of the same output.

The actionability tier-bias is also inverted from para-extract: where para-extract prefers to extract when ambiguous, this skill prefers to abstain when ambiguous. Reach for it in pipelines where the cost of a fabricated Project or a phantom Area exceeds the cost of a missed extraction.

---

## Where this fits in the pipeline

This skill is one of three focused passes that replace `para-extract`'s combined scan. The split exists because each entity tier has a different failure mode and benefits from being tuned independently:

1. **Pass 1a — `para-projects-areas` (this skill):** focused PA discovery with the precision rule (fail-closed on phantom commitments). Hardcodable because Projects and Areas are stable PARA primitives — Forte's definitions don't shift, and the field set is bounded. Aligned with `entity-project.md` and `entity-area.md`.
2. **Pass 1b — `para-resource-entities` (the parallel skill):** focused Resource discovery and entity typing, template-driven because the type set is open and the templates in `plugins/anansi.plugin/references/templates/` are the source of truth for what types exist and what their identity_fields look like. Emits its own paired `resources-toc.md` + `resources-typed.md` joined by the same `source_id`.
3. **Pass 2 — smart-brevity compression:** takes the typed PA + Resource extractions from Pass 1, walks back to the source, and produces compressed entity-shaped notes for each — using the same input-type vocabulary (email, meeting, newsletter, presentation, speech, social_media, workplace_memo, company_update, book, generic_prose) that the templates' `sources:` blocks reference.

Pass 1a and Pass 1b run **in parallel** on the same source. They share the `source_id` so Pass 2 can pull both together. The split into focused passes improves discovery (each pass is tuned for its own failure mode) and improves compression (each entity gets focused attention rather than one omnibus reduction).

This skill owns Pass 1a only. Resources and the smart-brevity compression are out of scope here.

---

## When to invoke

Trigger this skill when the user:

- Says "extract projects and areas", "find PA in this", "what projects and areas are here"
- Says "/para-pa", "PA extract", "PA only", "identify projects and areas", "pull PA from this", "PA toc"
- Pastes a document and asks specifically for the Project + Area extraction without Resources or Concepts
- Is feeding a downstream pipeline (vault canonicalization, commitment inventory, weekly review prep) where false-positive Projects or Areas would create downstream noise
- Wants a clean active-commitment list extracted from messy prose, in TOC form aligned with para-toc
- Asks "what am I committed to in this document"
- Asks "what projects does this touch and which areas do they belong to"

Do not invoke for:

- **Single-item judgment** ("is this a project or an area?") — use `para-classify`.
- **Full four-tier extraction** with Resources and Concepts — use `para-extract`.
- **Resource sub-typing** into person / organization / note — use `resource-typer`.
- **Vault writes**, note creation, edge linking — those are downstream.

---

## What this skill produces

**Two files**, both markdown, paired by a shared `source_id` and `generated_at` in their frontmatter so downstream code can rejoin them.

### File 1 — `projects-areas-toc.md` (the navigation spine)

Frontmatter:
```yaml
---
source_id: <stable hash of input>
generated_at: <ISO-8601 timestamp>
skill: para-projects-areas
parallel_pair: para-resource-entities
---
```

Then exactly two top-level sections in this order:

- `## 1. Projects` — bulleted list, each item shaped `- 1.N [project] <Canonical Name>` for top-level Projects in document order. Sub-projects nest as `- 1.N.M [project] <Sub-Project Name>` indented under their parent. Projects with an identified parent Area carry an `[area:2.M]` cross-reference tag *after* the type tag: `- 1.N [project] [area:2.M] <Canonical Name>`.
- `## 2. Areas` — bulleted list, each item shaped `- 2.N [area] <Canonical Name>`, in order of first appearance in the source.

If a section has no entries, render the heading followed by `_(none found)_` rather than an empty list.

### File 2 — `projects-areas-typed.md` (the entity detail)

Frontmatter matching File 1's `source_id` and `generated_at`:
```yaml
---
source_id: <same as File 1>
generated_at: <same as File 1>
skill: para-projects-areas
parallel_pair: para-resource-entities
---
```

Then:

- `## Projects` heading, followed by one `### project:<slug>` block per Project (in TOC address order, including sub-projects). Field set aligned with `entity-project.md` (v2.1+) identity_fields: **Address, Name, Status** (only when known_projects was provided), **Parent project** (only on sub-projects), **Parent area** (only when an `[area:2.N]` tag was applied), **Evidence, Goal, End date, Confidence, Vault match hint**, plus optional **Summary** and **Content** when the document supports a source-agnostic description.
- `## Areas` heading, followed by one `### area:<slug>` block per Area. Field set aligned with `entity-area.md` (v3.1+) identity_fields: **Address, Name, Status** (only when known_areas was provided), **Evidence, Standard, Owner** (when named, distinct from the user), **Description** (scope/subject of the responsibility), **Responsibility source, Confidence, Vault match hint**, plus optional **Summary** and **Content** when the document supports a source-agnostic description.
- `## Notes` heading, carrying extraction-level observations: ambiguities, ownership inferences, anti-pattern flags, project-candidates that didn't qualify, mislabeled inputs, inflection points, sub-project signals considered and rejected, borderline known-list matches the user should confirm.

If a section has no entries, render the heading followed by `_(none found)_`.

### Why two files

Splitting TOC from typed-fields lets downstream code do two different things with two different shapes: ingest the TOC for navigation and addressing, ingest the typed-fields for vault writes. The `source_id` join means they're never ambiguous. This pairs symmetrically with `para-resource-entities`, which emits its own `resources-toc.md` + `resources-typed.md` for the same source — together covering the full PARA scan that `para-extract` used to handle in one pass.

The Project-first ordering matches `para-toc`'s actionability-descending convention (Projects 1, Areas 2, Discussion 3, Resources 4, Concepts 5). Two-pass parsers and human readers alike are expected to resolve `[area:2.N]` cross-references against Section 2 below.

### The TOC line shape

Each entity's TOC bullet uses one of these forms:

```
- 1.1 [project] <Project Name>
- 1.1 [project] [area:2.1] <Project Name>             ← project nested under area 2.1
  - 1.1.1 [project] [area:2.1] <Sub-Project Name>     ← sub-project of 1.1, also under area 2.1
- 2.1 [area] <Area Name>
```

`[project]` and `[area]` are entity-type tags (parser-readable, mirroring para-toc). `[area:N.N]` is the cross-reference tag — present only on Projects that have an identified parent Area in this same TOC.

### Empty-but-structured is a valid output

If no Projects and no Areas qualify, **still emit both files** with their frontmatter intact and `_(none found)_` under each section heading. Downstream code must be able to distinguish:

- **Skill ran, found nothing** — both files present, frontmatter present, sections present with `_(none found)_` bodies.
- **Skill failed** — files missing or frontmatter missing.

A consumption-shaped document (a transcript the user is consuming, an article they're reading, an email about someone else's work) will frequently produce a fully-empty PA extraction. That's correct behavior, not a failure. The Notes section in File 2 explains why.

---

## Definitions (verbatim from Forte)

> **Project** — "Projects include the short-term outcomes you're actively working toward right now... Projects have a couple of features that make them an ideal way to organize modern work. First, they have a beginning and an end; they take place during a specific period of time and then they finish. Second, they have a specific, clear outcome that needs to happen in order for them to be checked off as complete, such as 'finalize,' 'green-light,' 'launch,' or 'publish.'" — *Building a Second Brain*, Ch. 5

> **Area** — "There are facets of your work and life that don't have a clear end goal or deadline. We call them 'areas of responsibility.' An area of responsibility has: a standard to be maintained, [and] an indefinite end date... Instead of a goal, an area of responsibility has a standard you're trying to maintain." — *The PARA Method*, Ch. 3

> **The responsibility test** — "The line between areas and resources is an opportunity to be completely honest with yourself: What is inside the circle of your responsibilities, which no one else is going to take care of for you, and what is outside?" — *The PARA Method*, Ch. 4

> **The drop test (the sharpest single-sentence Area gate)** — "If no one else would notice you dropped it, it's a resource, not an area." — *The PARA Method*, Ch. 7

> **The Dreams / Hobbies diagnostic** — "Goals with no projects are dreams; projects with no goals are hobbies." — *The PARA Method*, Ch. 11

These quotes are load-bearing. Every decision rule below traces back to one of them.

---

## Project markers (all three required)

A candidate becomes a Project only when **all three** markers are present in the input. Two-out-of-three does not promote. This is fail-closed.

### 1. Specific completable outcome

A verb-noun statement that can be checked off as done. Forte's exemplars: "finalize," "green-light," "launch," "publish." If you cannot write the outcome as a one-line finite-verb statement that someone could mark complete, the marker is not present.

Present:
- "Finalize the Q3 board deck"
- "Hire VP of Engineering"
- "Ship anansi v2"
- "Submit the grant application"

Not present:
- "Improve the deck" — "improve" has no completion criterion
- "Work on hiring" — work-verb without a target
- "Be more effective at fundraising" — quality-shaped, not outcome-shaped

### 2. Deadline or other finite timeframe

A hard date, a target quarter, a milestone deadline, or some endpoint that exists. Forte (*PARA Method*, Ch. 3): *"A deadline adds a time limit to achieving your goal. You don't want your efforts lingering on forever, never quite knowing whether you succeeded or failed."*

Present:
- "by May 15"
- "before the board meeting on Tuesday"
- "this quarter"
- "by end of 2026"
- "by Friday"

Not present:
- "someday"
- "eventually"
- "when I get around to it"
- (no temporal anchor at all)

**The date-attached heuristic:** if a candidate has a date or a finite timeframe attached to a specific outcome, it is almost certainly a Project, not an Area. Areas are indefinite by definition; the presence of an end-point is the strongest single Project signal in the document.

### 3. Active commitment

The user is working on it now — not aspirational ("I want to"), not dormant ("we used to"), not delegated-and-forgotten. Forte (*PARA Method*): the project list reflects the life you actually have, not *"the life you wish you had."* Aspirational framing kills this marker.

Present:
- "I've drafted..." / "we've started..." / "Sarah is reviewing..."
- "I'm working on..." / "we're shipping..."
- Discussion of active progress, edits, blockers, status

Not present:
- "I want to..." / "we should..." / "I've been thinking we should..."
- "Someday I'll..." / "It would be nice to..."
- "We used to..." / "Back when we were doing..."

### Forte's diagnostic: the indefinite-shaped trap

From *The PARA Method*, Ch. 3, on his own past project list (Hiring/staffing, Events, Research, Vacations, Professional development, Strategic planning):

> *"Not a single item on this list is a project, according to my definition. Projects are 'short-term efforts,' which means they need a clear end date. Does 'strategic planning' ever end for good? Is there ever a time when you can permanently cross off 'vacations' from your list? Hopefully not!"*

If the candidate name is shaped like "Strategic Planning," "Hiring," "Direct Reports," "Vacations," "Professional Development," "Marketing," "Research" — it is **not** a Project. It is the name of an Area. Trust the shape, not the user's framing.

---

## Area markers (both required)

A candidate becomes an Area only when **both** markers are present. Two-out-of-two. Fail-closed.

### 1. A standard to maintain (not a goal to reach)

Forte (*PARA Method*, Ch. 3): *"Instead of a goal, an area of responsibility has a standard you're trying to maintain."* The standard is the quality bar the user upholds indefinitely.

Forte's worked examples (Ch. 3):
- **Finances:** "pay all your bills on time and provide for your family's needs"
- **Parenting:** "spend quality time with your kids every evening and make sure they are always loved and protected"
- **Product Development:** "upgrading its speed and performance, fixing bugs quickly, and approving new updates to be released"
- **Health:** "maintain a level of fitness and wellness"

Present when the document signals: ongoing maintenance, "managing," "overseeing," recurring meetings, status-tracking, the user being the keeper of a quality bar. Verbs like *maintain*, *manage*, *oversee*, *steward*, *keep up*, *handle*.

Not present when: a one-time outcome is named, a deliverable is the focus, the work has a clear end-state.

### 2. Direct responsibility (the critical separator)

Forte's circle test, *The PARA Method*, Ch. 4:

> *"What is inside the circle of your responsibilities, which no one else is going to take care of for you, and what is outside?"*

The sharpest one-line operationalization, *The PARA Method*, Ch. 7:

> *"If no one else would notice you dropped it, it's a resource, not an area."*

This is the drop test. Apply it to every Area candidate before extracting: if the user vanished, would anyone notice this thing wasn't being upheld anymore? If no, it isn't an Area — it's an interest, and interests are out of scope here.

Three sources of responsibility (Forte, *PARA Method*, Ch. 3):

- **Official roles** — assigned/hired duties (job title, board seat, parental role).
- **Unofficial duties** — things the user has quietly committed to (an internal newsletter, mentoring, staff retreats).
- **Personal commitments** — standards the user holds themselves to (Health, Finances, Friendships, a craft).

The shared thread: **someone is on the hook**, and that someone is the user.

### Ownership signals (required for Area)

- First-person possessives: "our," "my," "we," "us," "I"
- First-person commitment verbs: "I'm responsible for," "I manage," "we run," "I'm on the board of"
- Role context: "as DCS board chair, I…", "the JERA team I lead"
- Ongoing-interaction patterns: weekly meetings, recurring reviews, status updates the user is in

### Third-party signals (kill the Area extraction)

- Third-person references without possessive ("Apple announced," "the article discusses")
- Reading-about / hearing-about framing ("I read that…", "Ron mentioned…")
- Named external entities the user isn't described as part of

When the document signals interest but not responsibility, **do not extract** as Area. The cost of false-positive Areas is high — they clutter the user's responsibility list and dilute the meaning of "on the hook."

---

## Naming heuristics

The shape of a name is itself a strong classifier. Use these to validate after applying the markers — and to canonicalize the name on emit.

Forte's metaphor, *The PARA Method*, Ch. 6: *"Projects sprint to a finish line; Areas run a marathon at a steady standard."* If the candidate name describes a sprint with a finish line, name it as a Project. If it describes a steady-state marathon, name it as an Area.

### Project names (verb-noun + date/scope)

Specific. Outcome-shaped. Often dated. Reads like something you can mark complete.

- ✅ "Q3 Board Deck"
- ✅ "Hire VP of Engineering"
- ✅ "Annual Fundraiser 2026"

Rule: if the candidate name is shaped like a verb-noun outcome with (or amenable to) a date or scope qualifier, it's a Project name. If you wrote it on a sticky note, you'd know when to take the sticky note down.

### Area names (role / domain / hat)

Vague. Indefinite. Noun-phrase. Reads like a folder label that will be valid in a year and in five years.

- ✅ "DCS Board Operations"
- ✅ "JERA Account Management"
- ✅ "Personal Health"

Rule: if the candidate name is shaped like a role you play, a domain you steward, a hat you wear, or a sphere of responsibility you maintain, it's an Area name. The name itself implies "ongoing."

### The shape mismatch flag

If a candidate passed the Project markers but has an Area-shaped name ("Strategic Planning," "Direct Reports"), reconsider — the markers were probably misread. Same in reverse: if a candidate passed the Area markers but has a Project-shaped name ("Hire VP of Engineering" — concrete outcome), reconsider — the markers were probably misread.

When the shape and the markers disagree, **trust the shape and re-walk the markers**. The shape carries Forte's accumulated genre knowledge; the markers carry the document's local evidence. Disagreement usually means the markers were applied loosely.

---

## Sub-projects and area cross-references

The TOC supports two kinds of structural relationships beyond the flat list:

### Sub-projects (decimal nesting under a parent project)

A sub-project gets a third-decimal address under its parent: `1.1.1`, `1.1.2`. Each sub-project is its own separate note in the vault — decimal addresses always denote separate notes (per para-toc's rule). The nesting is structural, not content-embedded.

**Sub-projects must satisfy ALL Project markers independently.** A sub-project still needs its own specific outcome, its own deadline (or a finite scope tied to the parent's deadline), and its own active commitment. A sub-project is not "a task inside a project" — that's still a task. A sub-project is a Project in its own right that happens to roll up under a larger one.

**Extract a sub-project relationship only when the document explicitly signals it.** Acceptable signals:
- "Phase 1 of <parent>", "Phase 2 of <parent>" — explicit phasing.
- "Stepping stone toward <parent>", "leading up to <parent>" — explicit precedence.
- "<Sub> is part of the larger <parent> effort" — explicit containment.
- "Workstream A of <parent>", "the design track of <parent>" — explicit decomposition.
- "Sub-project of <parent>", or any direct parent-child language.

**Do not infer sub-project relationships from topical adjacency.** Two Projects appearing in the same email, or about the same client, or on related deadlines, are not sub-projects of each other. Apply the precision rule: when the nesting signal is fuzzy, keep both projects flat at `1.1` and `1.2`, and Note that you considered nesting them.

**Sub-projects can themselves have sub-projects** (`1.1.1.1`), but in practice this is rare and warrants high scrutiny. If you find yourself going three decimals deep, re-examine — usually the right move is to flatten.

### The `[area:N.N]` cross-reference

When a Project (or sub-project) belongs to an Area also extracted in this TOC, append a cross-reference tag of the form `[area:N.N]` after the `[project]` tag, where `N.N` is the Area's decimal address (always in Section 2, so always shaped `2.N` or `2.N.N`).

Example layout (in `projects-areas-toc.md`):
```
## 1. Projects

- 1.1 [project] [area:2.1] Q3 Board Deck
  - 1.1.1 [project] [area:2.1] Sovereignty-Framing Slide

## 2. Areas

- 2.1 [area] DCS Board Operations
```

**Rules for the area tag:**

- The tag is only added when the Area is extracted in *this same TOC*. If the user references an Area not extracted (because responsibility wasn't signaled in this document), no tag is added — phantom area-pointers are not allowed.
- A Project may have at most one `[area:N.N]` tag. PARA's rule from Forte (*PARA Method*, Ch. 6): every project typically falls under one Area. If a Project genuinely spans two Areas, pick the dominant one and Note the secondary.
- A sub-project inherits its parent project's area-tag by default. Re-emit the tag on the sub-project line for parser clarity (don't make the parser reach upward).
- A Project with no identified parent Area in this TOC carries no area-tag — just `[project]`.

**Don't fabricate the parent Area to justify a tag.** If the document doesn't signal an Area's responsibility, the Project is emitted as `[project]` standalone, and a Note may flag that "this Project likely belongs to an Area that wasn't identified in this document."

---

## The precision rule

This is the single most important rule in the skill, and it inverts `para-extract`'s tie-breaker.

> **When ambiguous between extracting and emitting nothing, emit nothing.**
>
> When ambiguous between Project and "not-a-project," emit nothing.
> When ambiguous between Area and "not-an-area," emit nothing.

Where `para-extract` says *"prefer the more actionable tier"*, this skill says *"prefer no extraction over a fragile one."*

The rationale traces directly to *The PARA Method*: *"There is a common temptation to set up PARA to resemble the life you wish you had, instead of the life you actually have. Don't create a bunch of aspirational projects and goals that are merely wishful thinking."* (Ch. 8.)

Extracting a Project the user isn't actually committed to, or an Area the user isn't actually responsible for, is a tax on the user's active workspace. Better to miss a real one (the next document about it will catch it, especially if the user adds it to known_projects/known_areas) than to fabricate a phantom commitment.

When a candidate is borderline:

1. Drop the extraction.
2. Add a Note explaining what was seen and what specific evidence would tip it into qualifying next time.

The Note is the lossless fallback. The user reviews Notes, decides if it should be elevated, and if so, the next ingest catches it via the known-list match.

---

## Known projects/areas (optional input)

The caller may prepend a list of existing Projects and Areas from the user's vault. When present, the skill matches every extracted Project / Area against the known list and marks each as **existing** (matched a known one) or **new** (not in the list). Non-canonical references in the document get pulled toward the canonical vault form.

### Input format

If known lists are provided, they go at the top of the input, separated from the document by a `---` line:

```
known_projects:
  - project:q3-board-deck — Q3 Board Deck (active, deadline May 15)
  - project:vp-eng-search — Hire VP of Engineering (active, target Q4)
  - project:annual-fundraiser-2026 — Annual Fundraiser 2026 (active, October 18)

known_areas:
  - area:dcs-board-operations — DCS Board Operations
  - area:dcs-financial-management — DCS Financial Management
  - area:health — Personal Health
  - area:account-management-jera — JERA Account Management

---

<the document to extract from>
```

The format is loose — list items can be just `- slug — name` or include status hints in parentheses. The skill parses what's there and ignores what's missing.

### Matching logic

For every Project / Area extraction, walk the known list:

1. **Exact match** on canonical slug → mark `existing`, use the canonical slug.
2. **Strong semantic match** on the name (the document says "the board deck" but the known list has `project:q3-board-deck`) → mark `existing` *with confidence ≤ medium*, use the canonical slug, note the alias in evidence.
3. **No match** → mark `new`, propose a slug, suggest the canonical name in title-case.
4. **Weak/borderline match** — when the document mention could plausibly be the known item but evidence is thin → mark `new` and add a Note flagging the borderline so the user can confirm.

When in doubt between `existing` and `new`, prefer `new` with a Note. Better to surface a duplicate for the user to merge than to silently collapse a real new commitment into the wrong canonical slug.

### Why this matters

Without the known-list input, every extraction is a candidate-create. With it, you preserve vault canonicality (don't fragment `project:q3-board-deck` into `project:the-board-deck`) and you surface genuinely-new commitments the user may have implicitly accumulated. If no lists are provided, fall through normally — every extraction is implicitly `new`.

---

## Output format

Return **two markdown files**, both with frontmatter sharing the same `source_id` and `generated_at`. Both files are stable — downstream code parses both.

### File 1 — `projects-areas-toc.md`

```
---
source_id: <stable hash of input>
generated_at: <ISO-8601 timestamp>
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

- 1.1 [project] <Canonical Name>
- 1.1 [project] [area:2.N] <Canonical Name>             ← when this Project belongs to Area 2.N
  - 1.1.1 [project] [area:2.N] <Sub-Project Name>       ← sub-project of 1.1; inherits the area tag
- 1.2 [project] <Another Project>

## 2. Areas

- 2.1 [area] <Canonical Name>
- 2.2 [area] <Another Area>
```

### File 2 — `projects-areas-typed.md`

```
---
source_id: <same as File 1>
generated_at: <same as File 1>
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

### project:<slug>
- **Address:** 1.1
- **Name:** <canonical name>
- **Status:** <existing | new>            ← only when known_projects was provided
- **Parent project:** <address, e.g. "1.1"; omit unless this is a sub-project>
- **Parent area:** <address, e.g. "2.1"; omit if no area-tag>
- **Evidence:** "<verbatim quote from input>"
- **Goal:** <Forte's first defining trait — the specific completable outcome, verb-noun phrase>
- **End date:** <Forte's second defining trait — finite endpoint (date, quarter, milestone)>
- **Confidence:** <high | medium>
- **Vault match hint:** project:<slug>
- **Summary:** <optional — one source-agnostic paragraph; omit if the document doesn't support it>
- **Content:** <optional — 2–4 source-agnostic paragraphs; omit if the document doesn't support it>

## Areas

### area:<slug>
- **Address:** 2.1
- **Name:** <canonical name>
- **Status:** <existing | new>            ← only when known_areas was provided
- **Evidence:** "<verbatim quote from input>"
- **Standard:** <Forte's first defining trait — the quality bar the user is committed to upholding, in their own words from the source when possible>
- **Owner:** <person or role primarily responsible; omit if the user themselves is the sole owner or no separate owner is named>
- **Description:** <the scope and subject of the responsibility — what this area covers; distinct from Standard, which is the quality bar>
- **Responsibility source:** <official role | unofficial duty | personal commitment | unstated>
- **Confidence:** <high | medium>
- **Vault match hint:** area:<slug>
- **Summary:** <optional — one source-agnostic paragraph; omit if the document doesn't support it>
- **Content:** <optional — 2–4 source-agnostic paragraphs; omit if the document doesn't support it>

## Notes

- <ambiguities, ownership inferences, anti-pattern flags, project-candidates that didn't qualify, mislabeled inputs, inflection points, sub-project signals considered and rejected>
- <if known_projects/known_areas was provided, note any borderline matches the user should confirm>
```

### Template alignment

The field set above is aligned with the canonical anansi entity templates at `plugins/anansi.plugin/references/templates/`:

- `entity-project.md` (v2.1) identity_fields: `name`, `goal`, `status`, `end_date`, `summary`, `content`. The skill's `Goal` and `End date` map directly. `Status` maps when known_projects is provided. `Summary` and `Content` are optional and only emitted when the source supports a source-agnostic description.
- `entity-area.md` (v3.1) identity_fields: `name`, `standard`, `owner`, `description`, `summary`, `content`. The skill's `Standard`, `Owner`, and `Description` map directly. `Summary` and `Content` are optional. `Responsibility source` is skill-internal (the marker trace) and lives outside the template's identity_fields — it explains *why* the Area qualified rather than *what* the Area is, and downstream code can drop it before vault writes.

Read the templates before emitting File 2. They are the authoritative field set; any drift between this skill's spec and the templates should be resolved by updating both, not by emitting fields that don't match.

### Field rules

**`Status`:** appears on Projects and Areas **only** when the caller provided `known_projects` / `known_areas` lists. If no known-list was provided, omit the line entirely.

**`Confidence`:** this skill emits `high` or `medium` only. Anything that would be `low` is dropped per the precision rule and surfaced in Notes instead.

**`Parent project` / `Parent area`:** these fields exist in File 2 to make the relationships in File 1's TOC machine-readable without the parser having to walk indentation or parse type tags. When a sub-project nests under a parent in File 1 (`1.1` → `1.1.1`), File 2's `### project:<sub-slug>` block carries `Parent project: 1.1`. When a Project carries an `[area:2.N]` cross-reference tag in File 1, File 2's block carries `Parent area: 2.N`. Both fields are omitted when not applicable.

**`Summary` / `Content`:** these are template-aligned optional fields. Emit only when the source document gives enough material to write a source-agnostic description without invention. If the document is sparse — a one-line mention, a status update, a passing reference — omit both. Per Forte, the goal is the work the user actually has, not the work you wish they had; the same applies to entity descriptions.

**`Owner` / `Description` (Area only):** Owner is emitted when the document names a person or role primarily responsible for the Area, distinct from the user. If the user is the sole owner, omit Owner — that's the default and the `Responsibility source` field already captures it. Description is the scope/subject of the responsibility and complements Standard (which is the quality bar). When the document only supports one of the two, emit the one it supports.

**Source-id stability:** the `source_id` should be a deterministic hash of the input content (e.g., SHA-256 truncated to 12 chars). The same input run twice should produce the same `source_id`. This makes the two files safely re-joinable downstream.

### Slug convention

Slugs follow anansi's `match_key` algorithm: lowercase the canonical name, replace non-alphanumerics with spaces, collapse whitespace, join with hyphens.

- "Q3 Board Deck" → `q3-board-deck`
- "JERA Account Management" → `jera-account-management`
- "O'Brien Strategy Review 2026" → `o-brien-strategy-review-2026`

### Empty section rendering

If a section has no entries, render the section header with `_(none found)_` on its own line — no TOC bullets in File 1, no field blocks in File 2. Both files still emit with their frontmatter intact:

**File 1 (`projects-areas-toc.md`) for an empty extraction:**
```
---
source_id: <hash>
generated_at: <iso timestamp>
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

_(none found)_

## 2. Areas

_(none found)_
```

**File 2 (`projects-areas-typed.md`) for an empty extraction:**
```
---
source_id: <same hash>
generated_at: <same timestamp>
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

_(none found)_

## Areas

_(none found)_

## Notes

- Consumption-shaped document; no user-owned PA signals.
```

This is a successful run, not a failure.

---

## Confidence calibration

This skill emits two confidence levels only — high and medium. Low-confidence candidates are dropped and surfaced in Notes per the precision rule.

- **high** — Three or more markers present for the chosen tier (all three Project markers, or both Area markers + an explicit ownership signal), name is explicitly given, evidence is direct first-person framing.
- **medium** — Required markers fit but ownership is inferred rather than explicit, or name is implicit and pulled from context, or the document gives only a passing but credible mention with clear ownership.
- **(low — dropped)** — Markers are partial, ownership is speculative, name is fuzzy, or the extraction rests on inference rather than evidence. **The skill prefers silence and a Note over emitting a low-confidence extraction.** State this explicitly in any Note that explains a drop.

When emitting at medium confidence, the Note should call out what specifically would tip the extraction to high in a future ingest.

---

## Worked examples

### Example 1 — High-confidence Project with companion Area (cross-reference)

**Input:**
```
Subject: Q3 Board Deck — please review

Hi James, thanks for finalizing the agenda for next Tuesday's DCS board
meeting. I've drafted the Q3 board deck and would love your eyes on it
before May 15. — Sarah
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: a1c4e9f2b8d3
generated_at: 2026-05-02T15:30:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

- 1.1 [project] [area:2.1] Q3 Board Deck

## 2. Areas

- 2.1 [area] DCS Board Operations
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: a1c4e9f2b8d3
generated_at: 2026-05-02T15:30:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

### project:q3-board-deck
- **Address:** 1.1
- **Name:** Q3 Board Deck
- **Parent area:** 2.1
- **Evidence:** "I've drafted the Q3 board deck and would love your eyes on it before May 15"
- **Goal:** finalize Q3 board deck
- **End date:** May 15
- **Confidence:** high
- **Vault match hint:** project:q3-board-deck

## Areas

### area:dcs-board-operations
- **Address:** 2.1
- **Name:** DCS Board Operations
- **Evidence:** "next Tuesday's DCS board meeting"
- **Standard:** ongoing DCS nonprofit board governance — meetings, agenda, oversight
- **Responsibility source:** official role (first-person framing implied — user is being asked to review the deck as a board member)
- **Confidence:** medium
- **Vault match hint:** area:dcs-board-operations

## Notes

- DCS Board Operations classified as Area at medium confidence — ownership is inferred from the user being the deck reviewer, not explicitly stated. If user is not on the DCS board, drop the Area entry and remove the `[area:2.1]` tag from the Project.
- The May 15 date attached to a verb-noun outcome ("finalize the Q3 board deck") is the cleanest possible Project signal.
- The `[area:2.1]` cross-reference was added because both entities were extracted in the same TOC and the document signals the Project belongs to the Area (the deck *is* a deliverable of the board operations).
```

### Example 2 — High-confidence Area only (no Project)

**Input:**
```
Subject: Just checking in on JERA

James, no action needed — just a status update. The JERA engagement is
humming. Weekly syncs are productive, the team is shipping, and Mark
seems happy. I'll keep monitoring and flag anything that needs your
attention. — M
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 7f3a2b1d9e4c
generated_at: 2026-05-02T15:31:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

_(none found)_

## 2. Areas

- 2.1 [area] JERA Account Management
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 7f3a2b1d9e4c
generated_at: 2026-05-02T15:31:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

_(none found)_

## Areas

### area:account-management-jera
- **Address:** 2.1
- **Name:** JERA Account Management
- **Evidence:** "The JERA engagement is humming. Weekly syncs are productive..."
- **Standard:** ongoing client engagement health — weekly syncs, delivery cadence, stakeholder satisfaction
- **Responsibility source:** unofficial duty (user is the escalation point — "flag anything that needs your attention")
- **Confidence:** high
- **Vault match hint:** area:account-management-jera

## Notes

- No Projects extracted — "weekly syncs," "monitoring," and "keep monitoring" are all maintenance verbs, not outcome verbs. No deadline, no completable outcome named.
- High confidence on the Area: explicit ongoing-maintenance framing plus user as the escalation owner.
```

### Example 3 — Mislabeled "Project: Stay healthy" → Area

**Input:**
```
Project: Stay healthy

I want to be more consistent with sleep, exercise, and what I eat.
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 5c8e2d4a6b1f
generated_at: 2026-05-02T15:32:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

_(none found)_

## 2. Areas

- 2.1 [area] Personal Health
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 5c8e2d4a6b1f
generated_at: 2026-05-02T15:32:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

_(none found)_

## Areas

### area:health
- **Address:** 2.1
- **Name:** Personal Health
- **Evidence:** "Stay healthy ... be more consistent with sleep, exercise, and what I eat"
- **Standard:** maintain consistency in sleep, exercise, and nutrition
- **Responsibility source:** personal commitment
- **Confidence:** high
- **Vault match hint:** area:health

## Notes

- The user labeled this "Project" but the markers don't fit: "stay healthy" has no completion criterion (when are you done being healthy?), no deadline, no specific outcome. Reclassified to Area per Forte's list of indefinite-shaped names. Trust the markers, not the label.
- If the user wants this as a Project, they'd need to specify a measurable outcome and a deadline (e.g., "lose 15 lbs by July 1," "run a half marathon by October"). Specific health goals would appear as Projects in Section 1 with `[area:2.1]` tags pointing here.
```

### Example 4 — "I want to write a book someday" → nothing

**Input:**
```
I want to write a book someday — maybe a memoir, maybe a how-to
based on the consulting work. Not sure yet. Just keeping it on
the back burner.
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 9b4d6f3e8a2c
generated_at: 2026-05-02T15:33:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

_(none found)_

## 2. Areas

_(none found)_
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 9b4d6f3e8a2c
generated_at: 2026-05-02T15:33:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

_(none found)_

## Areas

_(none found)_

## Notes

- Project candidate seen but dropped per the precision rule. "Someday" + "back burner" defeats the deadline marker; "I want to" + "not sure yet" defeats the active-commitment marker. Specific outcome marker also fuzzy ("a book" — memoir or how-to undecided). Two of three Project markers absent; the candidate cannot promote.
- Per Forte (*PARA Method*, Ch. 8): *"Don't create a bunch of aspirational projects and goals that are merely wishful thinking."* This is the textbook pattern — the right move is no extraction.
- Would upgrade to Project on a future ingest if the user names: (a) a target outcome ("publish a 60K-word memoir"), (b) a deadline ("by end of 2027"), and (c) active first-step work ("draft chapter 1 by next month"). Until then, this lives outside PA.
```

### Example 5 — Sub-project nesting with explicit phasing signal

**Input:**
```
Subject: Anansi v2 plan — phased rollout

Quick framing: shipping anansi v2 is the umbrella effort. We're
breaking it into three phases:

  - Phase 1: typed-TOC pipeline (target: end of May)
  - Phase 2: daemon parser update for alpha addresses (target: mid-June)
  - Phase 3: full atomization rollout (target: end of June)

I'm driving all three. They roll up under the broader anansi
sovereignty work I've been maintaining.

— J
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 4e7c9a1b8d52
generated_at: 2026-05-02T15:34:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

- 1.1 [project] [area:2.1] Anansi v2
  - 1.1.1 [project] [area:2.1] Typed-TOC Pipeline
  - 1.1.2 [project] [area:2.1] Daemon Parser Update for Alpha Addresses
  - 1.1.3 [project] [area:2.1] Full Atomization Rollout

## 2. Areas

- 2.1 [area] Anansi Sovereignty Work
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 4e7c9a1b8d52
generated_at: 2026-05-02T15:34:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

### project:anansi-v2
- **Address:** 1.1
- **Name:** Anansi v2
- **Parent area:** 2.1
- **Evidence:** "shipping anansi v2 is the umbrella effort. We're breaking it into three phases"
- **Goal:** ship anansi v2
- **End date:** end of June (rolling up the three phase deadlines)
- **Confidence:** high
- **Vault match hint:** project:anansi-v2

### project:typed-toc-pipeline
- **Address:** 1.1.1
- **Name:** Typed-TOC Pipeline
- **Parent project:** 1.1
- **Parent area:** 2.1
- **Evidence:** "Phase 1: typed-TOC pipeline (target: end of May)"
- **Goal:** ship typed-TOC pipeline
- **End date:** end of May
- **Confidence:** high
- **Vault match hint:** project:typed-toc-pipeline

### project:daemon-parser-update-for-alpha-addresses
- **Address:** 1.1.2
- **Name:** Daemon Parser Update for Alpha Addresses
- **Parent project:** 1.1
- **Parent area:** 2.1
- **Evidence:** "Phase 2: daemon parser update for alpha addresses (target: mid-June)"
- **Goal:** update daemon parser to match alpha addresses
- **End date:** mid-June
- **Confidence:** high
- **Vault match hint:** project:daemon-parser-update-for-alpha-addresses

### project:full-atomization-rollout
- **Address:** 1.1.3
- **Name:** Full Atomization Rollout
- **Parent project:** 1.1
- **Parent area:** 2.1
- **Evidence:** "Phase 3: full atomization rollout (target: end of June)"
- **Goal:** complete atomization rollout
- **End date:** end of June
- **Confidence:** high
- **Vault match hint:** project:full-atomization-rollout

## Areas

### area:anansi-sovereignty-work
- **Address:** 2.1
- **Name:** Anansi Sovereignty Work
- **Evidence:** "the broader anansi sovereignty work I've been maintaining"
- **Standard:** ongoing maintenance of the anansi sovereignty initiative
- **Responsibility source:** personal commitment ("I've been maintaining")
- **Confidence:** medium
- **Vault match hint:** area:anansi-sovereignty-work

## Notes

- Sub-project nesting was extracted because the document gave an explicit phasing signal ("breaking it into three phases," "Phase 1," "Phase 2," "Phase 3") — exactly the kind of language that satisfies the sub-project criterion. Without that explicit signal, the three would have been emitted flat at 1.1, 1.2, 1.3.
- Each sub-project independently satisfies all three Project markers (specific outcome + deadline + active commitment), so each warrants its own decimal address and field block.
- All four projects carry `[area:2.1]` because the user said *"They roll up under the broader anansi sovereignty work."* The Area's confidence is medium because "I've been maintaining" is a soft ownership signal — credible, but not as explicit as a stated role.
```

### Example 6 — Known-list match (existing + new) plus a precision-rule drop

**Input:**
```
known_projects:
  - project:q3-board-deck — Q3 Board Deck (active, May 15)
  - project:vp-eng-search — VP Engineering Search (active, target Q4)

known_areas:
  - area:dcs-board-operations — DCS Board Operations
  - area:account-management-jera — JERA Account Management

---

Subject: Quick update on the board deck and a couple other things

James — quick rundown.

The board deck is shaping up. I've folded in Sarah's edits and the
sovereignty-framing slide we discussed. Should be in shape by Friday.

On the JERA side, I had a call with their team yesterday — they want
to expand the engagement scope. Sending notes separately.

Also wanted to flag: I've been thinking we should formalize a "Hawaiian
language" practice — I've been collecting articles and resources for
months but it's just a folder right now. Maybe time to elevate it.

— M
```

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 2d8f5a3b6c91
generated_at: 2026-05-02T15:35:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

- 1.1 [project] [area:2.1] Q3 Board Deck

## 2. Areas

- 2.1 [area] DCS Board Operations
- 2.2 [area] JERA Account Management
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 2d8f5a3b6c91
generated_at: 2026-05-02T15:35:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

### project:q3-board-deck
- **Address:** 1.1
- **Name:** Q3 Board Deck
- **Status:** existing
- **Parent area:** 2.1
- **Evidence:** "The board deck is shaping up... Should be in shape by Friday."
- **Goal:** finalize Q3 board deck (matched from known list)
- **End date:** Friday (consistent with known list's "active, May 15")
- **Confidence:** high
- **Vault match hint:** project:q3-board-deck

## Areas

### area:dcs-board-operations
- **Address:** 2.1
- **Name:** DCS Board Operations
- **Status:** existing
- **Evidence:** "the board deck" + "Sarah's edits and the sovereignty-framing slide we discussed" (Q3 board deck ties to DCS board governance)
- **Standard:** ongoing DCS nonprofit board governance — meetings, decks, oversight
- **Responsibility source:** official role (matched from known list)
- **Confidence:** medium
- **Vault match hint:** area:dcs-board-operations

### area:account-management-jera
- **Address:** 2.2
- **Name:** JERA Account Management
- **Status:** existing
- **Evidence:** "I had a call with their team yesterday — they want to expand the engagement scope"
- **Standard:** ongoing client relationship management — calls, scope tracking, engagement health
- **Responsibility source:** unofficial duty (user is the JERA point person)
- **Confidence:** high
- **Vault match hint:** area:account-management-jera

## Notes

- "Hawaiian language" was considered as a candidate Area and dropped per the precision rule. The user explicitly says *"it's just a folder right now"* and is *thinking about* elevating it. This is the inflection point Forte names: a topic-of-interest becomes an Area when responsibility is taken on, not when interest accumulates. Without the elevation, no Area extraction.
  - **Would extract on a future ingest if:** the user names a maintained standard (e.g., "30 minutes of practice daily") and an explicit responsibility framing. Adding `area:hawaiian-language` to known_areas after that decision would catch the next reference automatically.
- "Sovereignty-framing slide" was considered as a Project candidate and dropped — it's a part of the parent Q3 Board Deck, not a standalone Project. Without an explicit phasing or sub-project signal, the precision rule says: don't nest, don't emit. It's a task inside `project:q3-board-deck`, not a sibling at `1.1.1`.
- The DCS Board Operations Area was emitted at medium confidence even though it's in the known list — the document gives only an indirect tie via "the board deck." If the known list says it's the user's official role, treat as high.
- Sarah is named once in passing; she's not a PA entity — she'd be a Resource at most, which is out of scope here.
```

### Example 7 — Consumption-shaped document → fully empty PA (still emit both files)

**Input:** A YouTube transcript of a guest speaker's talk titled "Why Sovereign AI Matters." User is consuming this for reference — no first-person ownership signals anywhere in the transcript.

**File 1 — `projects-areas-toc.md`:**
```
---
source_id: 6a1c4f8d2e95
generated_at: 2026-05-02T15:36:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## 1. Projects

_(none found)_

## 2. Areas

_(none found)_
```

**File 2 — `projects-areas-typed.md`:**
```
---
source_id: 6a1c4f8d2e95
generated_at: 2026-05-02T15:36:00-10:00
skill: para-projects-areas
parallel_pair: para-resource-entities
---

## Projects

_(none found)_

## Areas

_(none found)_

## Notes

- Consumption-shaped document. The user is not described as producing or being responsible for anything in this transcript — they're consuming it. No first-person possessives, no commitment verbs, no role context tying the user to any of the named entities.
- The talk itself, the speakers, and the topics discussed would all classify as Resources or Concepts in `para-extract`. They are out of scope here.
- This is a clean, valid empty PA result, not a failure. Both files are still emitted with frontmatter intact and `_(none found)_` bodies — that's the contract for downstream code to recognize "skill ran, found nothing."
```

---

## Anti-patterns to flag

When the input or framing exhibits any of these, surface a warning under the relevant section or in Notes.

### 1. Extracting a Resource or Concept

This skill's scope is Projects and Areas only. Named external people, organizations, articles, papers, talks, topics-in-passing, and tag-shaped ideas do **not** belong in the output. If you're tempted to extract one, the right move is to drop it silently and rely on `para-extract` for the broader scan.

**Tell:** the candidate is third-party, topic-shaped, or reference-material-shaped.

### 2. Extracting an action item as its own Project

Tasks inside a Project are usually parts of that Project, not standalone Projects. "Send Sarah the slide draft" is a task within `project:q3-board-deck`, not its own Project at `1.1.1`. A sub-project decimal is reserved for a Project that independently satisfies all three Project markers — outcome, deadline, active commitment — not for a checkbox.

**Tell:** the candidate is verb-noun-shaped but tiny and parented under a real Project.

### 3. Promoting an indefinite-shaped string to Project

"Strategic Planning," "Hiring," "Direct Reports," "Vacations," "Professional Development," "Marketing," "Research" — Forte explicitly lists these as the canonical mistake (*PARA Method*, Ch. 2). They're indefinite by shape; they're Areas, not Projects.

**Tell:** the candidate name is a domain or activity category, not a verb-noun outcome with a date.

### 4. Promoting a topic of interest to Area without responsibility

"AI sovereignty," "Hawaiian language," "tactical empathy" — these are topics. They become Areas only when the user signals they are *responsible* for upholding a standard in that domain. Mere collection of material doesn't elevate. Mere interest doesn't elevate. Apply the drop test (*PARA Method*, Ch. 7): if no one else would notice you dropped it, it's not an Area.

**Tell:** the document signals interest, learning, or material-collection — but no maintenance commitment.

### 5. Auto-elevating a "thinking-about-elevating" candidate

When the user says they're considering formalizing a practice but haven't yet, the skill must **not** elevate. The decision is the user's, and the document's signal is explicitly "not yet." Stay outside PA and Note the inflection point.

**Tell:** verbs like "thinking about," "maybe time to," "I should formalize," "considering elevating."

### 6. Aspirational-life PA

Per Forte (*PARA Method*, Ch. 8): *"There is a common temptation to set up PARA to resemble the life you wish you had, instead of the life you actually have."* If a candidate represents the wish, not the work, drop it.

**Tell:** language of aspiration, intention, or wishlisting without active progress markers.

### 7. Over-nesting sub-projects

Two Projects appearing in the same document, or about the same client, or with related deadlines, are not automatically sub-projects of each other. Sub-project nesting requires an explicit phasing / containment / decomposition signal in the document. Inferring a hierarchy from topical adjacency manufactures structure that the user never declared.

**Tell:** the only thing connecting candidate sub-projects is shared topic or shared timeframe — no explicit "phase," "part of," "stepping stone," "workstream," or parent-child language.

### 8. Phantom area cross-references

Tagging a Project with `[area:N.N]` when the referenced Area wasn't actually extracted in this same TOC. The cross-reference must always resolve to a real address in Section 2 of this output. If you believe a Project belongs to an Area that the document doesn't establish responsibility for, drop the tag and Note the suspected (but unidentified) parent — don't fabricate an address.

**Tell:** the Area would have to be invented or pulled from outside-the-document context to justify the tag.

---

## Edge cases

### "It's a Project AND an Area at the same level"

The Project belongs *under* the Area as a member, not as a peer. Example: "Hire VP of Engineering" is a Project; "Direct Reports / Team Building" is an Area. Both extract — `1.1 Hire VP of Engineering [project] [area:2.1]` and `2.1 Direct Reports [area]`. The cross-reference tag captures the relationship; the addresses keep them distinct.

### "Looks like an Area but might be a Project"

Apply the deadline test. If there's an end date or finite timeframe attached to a specific outcome, it's a Project. If the document shows ongoing maintenance with no end-state, it's an Area. Forte's diagnostic list (Strategic Planning, Hiring, Direct Reports, Vacations) applies: indefinite shape always wins over project-shaped framing.

### "User is *thinking about* elevating something to an Area"

Don't auto-elevate. The document signals consideration, not commitment. Drop the candidate and Note the inflection — when the user makes the decision, they can add it to known_areas and the next ingest will catch it.

### "Known list says Project X is active but the document suggests it's done"

Flag the contradiction in Notes. Don't silently drop the existing match, and don't fabricate a status change. The user resolves it.

### "Known list has a Project the document doesn't mention"

Don't fabricate evidence. Only emit extractions for entities the document actually references. The known list is for canonicalization, not for hallucinating presence.

### "Same entity appears multiple ways in the document"

Deduplicate. "the deck," "the Q3 deck," "the board deck" → one Project at one address with the most-specific name as canonical and the strongest evidence quote.

### "Two candidates share the same slug"

Disambiguate the canonical name. Note the disambiguation choice in Notes. Common case: two different Q3 board decks across two different orgs.

### "Document references the user themselves"

Don't extract the user. Skip first-person references that name the user.

### "An Area is mentioned but no responsibility is signaled in this document"

If the user has mentioned in the past that they're on a certain board, but *this particular document* is silent on ownership, you can still extract at medium confidence using known_areas as the responsibility evidence. If known_areas is not provided and the document gives no ownership signal, drop the candidate.

### "A sub-project is signaled but the parent isn't extracted in this document"

The sub-project is a real Project candidate in its own right (it has its own outcome, deadline, and active commitment). Emit it at the top level as `1.N` rather than nesting under a phantom parent. Note in `## Notes` that the document references a parent project that wasn't extracted here, with the parent's name. If the user adds the parent to known_projects on a future ingest, both will canonicalize and the next document with the same nesting signal will produce `1.M.1` correctly.

### "Two Projects share an area-tag but neither is a sub-project of the other"

Multiple flat Projects can each carry the same `[area:2.N]` tag. They're sibling Projects under the same Area, not sub-projects of each other. Emit `1.1 [project] [area:2.1]` and `1.2 [project] [area:2.1]` and let the area cross-reference do the relational work.

---

## What this skill does NOT do (out of scope)

- **Does not extract Resources.** Named people, organizations, articles, talks, papers, and reference material are silently skipped. Use `para-extract` for the full four-tier scan.
- **Does not extract Concepts.** Tag-shaped ideas, topics-in-passing, and conceptual references are silently skipped. Use `para-extract`.
- **Does not extract Archive items.** Inactive Projects/Areas are noted in Notes if conspicuous, but not emitted as Archive entries. Use `para-extract` if an Archive view is needed.
- **Does not classify a single item.** For one-off "is this a Project or an Area?" judgment, use `para-classify`.
- **Does not look up entities in the vault** beyond the known_projects/known_areas inputs.
- **Does not create or modify notes.** Output is structured extraction; routing is downstream.
- **Does not produce a full five-bucket anansi TOC.** This skill emits a two-section TOC (Projects, Areas) only, in its own paired-file format. The full TOC with Discussion, Resources, and Concepts is `para-toc`'s job. Output here is parser-compatible with the para-toc shape, but a strict subset.
- **Does not handle file I/O.** User pastes content; skill processes it.
- **Does not auto-promote** Resources or Concepts to Areas. The responsibility-elevation decision is the user's, not the skill's.
- **Does not emit low-confidence extractions.** Per the precision rule, low-confidence candidates are dropped and surfaced in Notes.
- **Does not infer sub-project hierarchies from topical adjacency.** Sub-project nesting requires an explicit phasing / containment signal in the document.
- **Does not emit `[area:N.N]` cross-references to Areas not extracted in this same TOC.** Phantom area-pointers are not allowed.
- **Does not wrap output for daemon ingestion or write to anansi.** Output is two paired markdown files (`projects-areas-toc.md` + `projects-areas-typed.md`) the caller can compose into a larger TOC. Daemon ingestion and vault writes are downstream concerns handled by `para-toc` and the anansi capture/ingest tools.

---

## Process

The build order is internal: assemble Areas first to reserve their `2.N` addresses, then assemble Projects with `[area:2.N]` cross-references resolved, then emit two paired files in the para-toc-aligned 1 → 2 order.

When invoked:

1. **Read the canonical entity templates** at `plugins/anansi.plugin/references/templates/entity-project.md` (v2.1+) and `entity-area.md` (v3.1+). These are the authoritative field set for File 2 output blocks. If your understanding of the field set drifts from what the templates say, trust the templates.
2. **Parse the input.** If `known_projects:` and/or `known_areas:` blocks appear at the top (separated from the document by `---`), extract those lists. Otherwise, proceed without them.
3. **Read the document fully.** Don't classify on partial context.
4. **Identify candidate entities.** Walk the document, flagging proper nouns, named efforts, named subjects, role mentions, and outcome-shaped phrases. Be liberal at this step — narrowing happens in marker-walking.
5. **Walk Area markers first (both required, fail-closed)** for every candidate.
   - Standard to maintain? (Ongoing-maintenance verbs, quality bar implied or stated.)
   - Direct responsibility? (First-person ownership signals, role context, "no one else would notice if you dropped it.")
   - Both present → Area candidate. One or zero → not an Area.
   - Doing Areas first lets us assign their `2.1`, `2.2`, ... addresses before any Project tries to reference them.
6. **Walk Project markers (all three required, fail-closed)** for the remaining candidates and any Area-rejects worth re-checking.
   - Specific completable outcome? (Verb-noun, checkable as done.)
   - Deadline or finite timeframe? (Hard date, target quarter, milestone.)
   - Active commitment? (User is working on it now, not aspirational, not dormant.)
   - All three present → Project candidate. Two or fewer → drop.
7. **Determine sub-project relationships.** For each Project candidate, check the document for explicit phasing / containment / decomposition / "stepping stone" / "part of" language pointing at another Project candidate. If signaled, mark the relationship for nesting under the parent's address. If only adjacency or topical overlap, keep both flat and Note that nesting was considered and rejected.
8. **Determine area cross-references.** For each Project candidate, check whether the document signals it belongs to one of the extracted Areas. If yes, attach the Area's address as `[area:2.N]`. If the document references an Area not extracted (because responsibility wasn't signaled here), do not attach a tag — Note instead that the Project likely belongs to an unidentified Area.
9. **Apply the precision rule.** For any candidate that survived markers but is borderline (markers fit weakly, ownership inferred thinly, name fuzzy), drop the extraction and note in `## Notes`. Emit only high or medium confidence.
10. **Validate names against the naming heuristics.** If a Project candidate has an Area-shaped name (or vice versa), re-walk the markers — there's likely a misread.
11. **Match against known_projects / known_areas if provided.** Mark `existing` (canonical match), `new` (no match), or `new + Note` (borderline).
12. **Compute slugs** using the lowercase / non-alphanumeric → space / collapse → hyphenate algorithm.
13. **Deduplicate.** One canonical entry per entity, most-specific name as canonical, strongest evidence quote.
14. **Assign addresses.**
    - Areas get `2.1`, `2.2`, … in extraction order.
    - Top-level Projects get `1.1`, `1.2`, … in extraction order.
    - Sub-projects get `1.N.1`, `1.N.2`, … under their parent's address.
15. **Compute `source_id` and `generated_at`.** `source_id` is a deterministic hash of the input (e.g., SHA-256 truncated to 12 hex chars). `generated_at` is an ISO-8601 timestamp with timezone offset. Both go in the frontmatter of both output files.
16. **Emit File 1 (`projects-areas-toc.md`).** Frontmatter, then `## 1. Projects` with bulleted TOC lines (sub-projects nested as indented bullets, `[area:2.N]` cross-reference tags applied where relevant), then `## 2. Areas` with bulleted TOC lines. Empty sections render as `_(none found)_` under the section header.
17. **Emit File 2 (`projects-areas-typed.md`).** Same frontmatter, then `## Projects` with `### project:<slug>` field blocks in TOC address order (top-level, then sub-projects), then `## Areas` with `### area:<slug>` field blocks, then `## Notes`. Field set must match `entity-project.md` v2.1+ for projects (Goal, End date, Status, optional Summary/Content) and `entity-area.md` v3.1+ for areas (Standard, Owner, Description, optional Summary/Content). Empty sections render as `_(none found)_`.
18. **Verify file pairing.** Both files must share the exact same `source_id` and `generated_at` strings. Both files emit even on empty extractions — frontmatter intact, sections present, `_(none found)_` bodies. This is the contract for downstream code to distinguish "skill ran, found nothing" from "skill failed."
19. **Add Notes** (in File 2 only) for: ambiguities, ownership inferences, anti-pattern flags, candidates dropped per the precision rule (with what would tip them next time), sub-project nesting considered and rejected, inflection points, mislabeled inputs, borderline known-list matches the user should confirm, and any `[area:...]` references the parser should expect to see.

Always quote evidence verbatim. Don't paraphrase the document into the Evidence field.

---

## Why this matters

PARA's value comes from a clean active-commitment list — Projects and Areas are the two most actionable tiers, and they're the ones the user looks at to answer *"what am I on the hook for right now?"* Both `para-extract` and `para-classify` lean toward extracting when ambiguous, because the cost of a missed Resource or a misclassified Concept is small. But for Projects and Areas, the cost asymmetry flips: a phantom Project clutters the active workspace, dilutes the meaning of "project," and erodes trust in the list; a phantom Area inflates the user's responsibility surface, manufacturing a standard they never agreed to uphold. This skill exists for the pipelines where that asymmetry matters most — vault canonicalization, weekly review prep, commitment inventory, anything that feeds a list the user actually consults to plan their week. The two-file output (TOC + typed-fields, joined by `source_id`) makes the Project-Area relationship machine-readable without sacrificing the precision rule, and pairs symmetrically with `para-resource-entities` so the full PARA scan that `para-extract` used to handle in one pass is now three focused passes (PA discovery, Resource discovery, smart-brevity compression) instead of one omnibus reduction. Field-set alignment with the canonical anansi entity templates (entity-project.md v2.1+, entity-area.md v3.1+) means the skill's emitted blocks slot directly into the vault without a translation layer. Reach for it when over-identification is the failure mode you're tuning against, and accept that empty output is sometimes the most honest result.

## Done

- [ ] `projects-areas-toc.md` and `projects-areas-typed.md` both exist
- [ ] TOC has `## 1. Projects` and `## 2. Areas` sections
- [ ] All entries have decimal addresses (1.1, 1.2, ... 2.1, 2.2, ...)
- [ ] All entries have type tags (`[project]` or `[area]`)
- [ ] `[area:2.N]` cross-references resolve to real addresses in Section 2
- [ ] Sub-projects nest under parent with third-decimal addresses (1.1.1, 1.1.2)
- [ ] Empty sections render as `_(none found)_` rather than being omitted
- [ ] Both files share the same `source_id` and `generated_at` in frontmatter

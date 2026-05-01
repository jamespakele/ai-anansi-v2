---
name: para-extract
description: >
  Scan a document — email, thread, meeting summary or transcript, YouTube
  summary or transcript, article, prose — and extract Projects, Areas,
  Resources, and Concept tags using Tiago Forte's PARA framework from the
  user's actionability perspective. Optionally accepts known_projects and
  known_areas blocks at the top of the input (separated by triple-dash) to match
  extractions against the user's vault and mark each as `existing` or
  `new`. Resources stay flat at this layer; sub-typing is deferred. Output
  is structured markdown with vault_match_hint fields ready for downstream
  lookup-or-create. Use when user says "extract PARA from this", "what
  projects/areas/resources are in here?", "/para-extract", or as the first
  step of an atomization pipeline. Grounded in *Building a Second Brain*
  (2022) and *The PARA Method* (2023).
argument-hint: "[document content; optionally prefixed with known_projects/known_areas blocks]"
---

# para-extract

A document scanner. Read an input — typically one of:

- **Email** (single message)
- **Email thread** (multi-message exchange)
- **AI meeting summary** (structured summary produced by a meeting tool)
- **Meeting transcript** (raw or lightly-processed verbatim transcript)
- **YouTube summary** (structured summary of a video)
- **YouTube transcript** (raw timestamped transcript)
- **Research article or paper**
- **Any prose document**

— and emit a structured list of every entity worth tracking in the user's vault, classified by PARA tier from the user's actionability lens, plus a Concepts list (tag-shaped references like `#sovereign-ai`, `#tactical-empathy`).

**Scope for now:** identify Projects, Areas, Resources, and Concepts. Resources are emitted as a flat list — no further sub-typing into person/organization/note. That's a deferred concern handled by a later skill in the pipeline. This skill answers two questions per candidate: *Project, Area, Resource — or Concept (a tag, not a note)?*

This is **not** a per-item judgment skill (use `para-classify` for that). This skill processes one document and surfaces many entities. It composes on top of `para-classify`'s decision rules but applies them in bulk.

---

## When to invoke

Trigger this skill when the user:

- Pastes an email, meeting summary, transcript, or prose document and asks to extract PARA entities
- Says "extract PARA from this", "what projects are in here?", "what areas does this touch?", "what resources are mentioned?"
- Says "/para-extract" or any extraction phrasing
- Is starting an atomization workflow that will downstream into vault lookup-or-create

Do not invoke for: classifying a single thing (use `para-classify`), organizing an existing vault, or filing a note — those are separate concerns.

---

## What the skill produces

Four lists, in order, plus a Notes section:

1. **Projects** — *the user's* active efforts mentioned in the document. Each has a specific outcome and ideally a deadline.
2. **Areas** — *the user's* ongoing responsibilities mentioned. Each has a standard maintained over time.
3. **Resources** — every other named entity worth tracking that doesn't belong to a current Project or Area. Identified flat. Sub-typing is deferred to a later skill.
4. **Concepts** — tag-shaped ideas mentioned in the document. *Not* extracted as their own notes; emitted as tags to be applied to whatever synergy note this document becomes (the discussion, email-exchange, youtube-chapter, etc.).
5. **Notes** — extraction-level observations: ambiguities, ownership inferences, anti-pattern flags.

The output is structured markdown designed for downstream code to parse and act on (lookup in vault, append-or-create, link as evidence, attach tags).

### Topics vs. Concepts (the key distinction)

A document has **one topic** (its subject/headline) and may invoke **many concepts** (ideas floated within).

- **Topic** = the headline/subject of the document. Singular. Becomes a `note`-shaped Resource if it's a substantial named subject. Example: a meeting titled *"Q3 Strategy Review"* — topic is `Q3 Strategy Review`.
- **Concept** = an idea referenced in passing while discussing the topic. Plural. Emitted as tags; never a standalone note. Example: in a discussion *about* "Q3 Strategy Review," concepts mentioned might include `#runway`, `#hiring-plan`, `#north-star-metric`.

Same physical idea can show up as a topic in one document and a concept in another. Sovereign AI is the *topic* of a discussion specifically about sovereign AI; the same phrase is a *concept* (tag) when mentioned in passing in a discussion about something else.

### Concept vs. Area (the responsibility test)

This is the harder call. *The PARA Method* gives the answer (Forte, Ch. 3):

> "There is a big difference between things you are **directly responsible for** and things you are **merely interested in**."

A topic becomes an **Area** when the user has *responsibility* for it — official role, unofficial duty they've taken on, or personal commitment to uphold a standard. A topic stays a **Concept (tag)** when the user is *merely interested* — tracking, learning, curious, but not on the hook for any standard.

Forte's example: a public health professor responsible for nutrition research → "Nutrition" is an **Area** for them. An anthropology student with a passing interest in nutrition → "nutrition" is a **Resource / tag** for them. **Same topic, different relationship.**

When extracting, ask:
- Does the document signal that the user has a *standard to maintain* in this domain? (Then Area.)
- Does the user reference it as something they're *responsible for* — a role, a duty, a commitment? (Then Area.)
- Or is it just a theme floated in conversation? (Then Concept tag.)

When ambiguous, **default to Concept** and flag in Notes — areas are weighty commitments and false-positives clutter the user's responsibility list.

---

## Known projects/areas (optional input)

The caller may prepend a list of existing Projects and Areas from the user's vault. When present, the skill matches every extracted Project/Area against the known list and marks each as **existing** (matched a known one) or **new** (not in the list). Non-canonical references in the document get pulled toward the canonical vault form.

### Input format

If the user provides known lists, they go at the top of the input, separated from the document by a `---` line:

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

When in doubt, prefer `existing` only on clear matches; weak matches go to `new` with a Note flag for the user to confirm.

### Why this matters

Without the known-list input, every extraction is a candidate-create. With it, you preserve vault canonicality (don't fragment `project:q3-board-deck` into a duplicate `project:the-board-deck`) and you surface genuinely-new commitments the user may have implicitly accumulated.

If no lists are provided, fall through normally — every extraction is implicitly `new` (and the downstream pipeline does its own vault lookup).

---

## The actionability lens (and why it matters)

Forte's PARA classifies by **actionability *to the user*** — not by what the entity is in the abstract.

- *"Apple's iPhone 18 launch"* — Apple's project, but to the user it's a **Resource**. The user isn't the one launching.
- *"Our Q3 board deck for DCS"* — *user's* Project (first-person possessive, named outcome).
- *"The Continest organization in Phoenix"* — third-party org, **Resource**. The user is referencing them, not running them.
- *"DCS board"* — *user's* Area if user is on the board (first-person ownership signals); otherwise Resource.

When the document signals user ownership/responsibility, classify into Project or Area. When it doesn't, classify into Resource. **When ambiguous, default to Resource** — it's the catchall — and note in the Notes section what would tip the decision.

### User-ownership signals

- First-person possessives: "our", "my", "we", "us", "I"
- First-person commitment verbs: "I need to", "we're working on", "I'm responsible for"
- Names that are clearly the user's company / org / project (if the document gives this context)
- Calendar / role context: "as DCS board chair, I…"

### Third-party signals (→ Resource)

- Third-person references without possessive: "Apple announced", "the article discusses"
- Reading-about / hearing-about framing: "I read that…", "Ron mentioned…"
- Named external entities the user isn't described as part of

When the document doesn't make ownership clear, prefer Resource and flag the ambiguity in Notes.

---

## Decision rules per tier

For every candidate entity, walk the four-question test (Project? → Area? → Resource? → Archive?). Stop at first yes. Sharpened from *The PARA Method* (Forte, 2023):

### Project markers (Q1) — three required

A Project must have **all three**:

1. **A specific outcome** — something you can mark "complete." Verb-noun shape: "publish blog post," "hire VP," "ship v2," "finalize Q3 deck." If you can't write the outcome in one sentence with a finite verb, it's not a Project.
2. **A deadline or other timeframe** — finite, not indefinite. Hard date, target quarter, or some endpoint that exists. *Not* "someday."
3. **Active commitment** — being worked on now. Not aspirational ("I want to") and not dormant ("we used to").

The diagnostic Forte gives in *PARA Method* (Ch. 3): if you list "Strategic Planning," "Hiring," "Direct Reports," or "Vacations" as projects, **none of those are projects** — they're indefinite areas. *"Does strategic planning ever end for good? Hopefully not!"*

### The Dreams / Projects / Hobbies test

Forte's three-way diagnostic for things that look project-shaped:

- **Goal but no active work** → it's a **Dream** (wishlist, aspiration). Don't extract as Project. Note in Notes if the user might want to elevate it.
- **Active work but no specific goal** → it's a **Hobby** (just for fun). Don't extract as Project; consider Area or Resource depending on commitment level.
- **Goal AND active work** → genuine Project. Extract.

### Area markers (Q2) — two required, one critical

An Area requires:

1. **A standard to maintain** — not an outcome to reach. Forte's examples (Ch. 3 of *PARA Method*):
   - **Finances:** "pay all bills on time and provide for your family's needs"
   - **Parenting:** "spend quality time with kids every evening; ensure they're loved and protected"
   - **Product Development:** "upgrade speed/performance, fix bugs quickly, approve releases"
   - **Health:** "maintain a level of fitness and wellness"
   The standard is the *quality bar* the user is committed to upholding indefinitely.
2. **Direct responsibility** — the critical separator from Resources. Ask: *Is the user **directly responsible** for this, or **merely interested** in it?* Forte's litmus example:
   - A public health professor responsible for nutrition research → **Area**.
   - An anthropology student curious about nutrition → **Resource (tag)**.
   - Same topic, completely different classification, all driven by responsibility.

### What raises a topic to Area status

Forte (*PARA Method* Ch. 3): areas are **"the roles you play"** or **"the hats you wear."** Three sources of an Area:

- **Official roles** — assigned/hired duties (your job title's responsibilities, board seat, parental role).
- **Unofficial duties you've taken on** — "Company Newsletter," "Mentoring," "Staff Retreats" — things you do because you've quietly committed.
- **Personal commitments** — your own standards (Health, Finances, Friendships, a craft you maintain).

The shared thread across all three: **someone (you) is on the hook**. Without the user, no one else picks it up.

### The "no one else will take care of it" test

Forte's most powerful diagnostic (Ch. 4): *"What is inside the circle of your responsibilities, which no one else is going to take care of for you, and what is outside?"*

If the user walked away, would the standard collapse? If yes → Area. If no → Resource or Concept tag.

### Resource markers (Q3) — the catchall

If not Project and not Area:
- It's a topic of interest, a person/org reference, a concept-with-substance, a referenced document, or anything else worth tracking.
- Forte's filter (*PARA Method* Ch. 4): not "is this interesting?" but "**is this useful?**" Resources earn their place by potential utility, not curiosity alone.
- **Resources are flat at this layer.** No sub-typing into person/organization/note. A later skill picks the entity_type using anansi's templates.

### Archive markers (Q4) — rare in document extraction

Archive surfaces only when the document explicitly says something is no longer active ("we used to," "after we discontinued," "pre-merger," etc.). Most documents are about live commitments, so Archive extractions are rare.

### The ranking rule when ambiguous

Per Forte's actionability hierarchy: Project > Area > Resource > Archive. **When confidence is split**, prefer the more actionable tier — but with one important exception introduced by *PARA Method*:

> *"There is a common temptation to set up PARA to resemble the life you wish you had, instead of the life you actually have."*

Don't promote ambiguous things to Project just because they sound exciting. The cost of false-positive Projects is high — they clutter the active list and dilute the meaning. **When unsure between Project and Resource, prefer Resource.** When unsure between Area and Concept tag, prefer Concept tag (the responsibility threshold for Area is meaningful).

---

## Output format

Return a single markdown document with this exact structure. Keep field names and ordering consistent — downstream code parses this.

```
# PARA extraction: <source title, filename, or "this input">

## Projects

### project:<slug>
- **Name:** <canonical name>
- **Status:** <existing | new>            ← only when known_projects was provided
- **Evidence:** "<verbatim quote from input>"
- **Outcome:** <stated outcome verb-phrase, or "unstated">
- **Deadline:** <stated date, or "unstated">
- **Confidence:** <high | medium | low>
- **Vault match hint:** project:<slug>

## Areas

### area:<slug>
- **Name:** <canonical name>
- **Status:** <existing | new>            ← only when known_areas was provided
- **Evidence:** "<verbatim quote from input>"
- **Standard:** <the standard the user is upholding, in their own words from the doc>
- **Responsibility source:** <official role | unofficial duty | personal commitment | unstated>
- **Confidence:** <high | medium | low>
- **Vault match hint:** area:<slug>

## Resources

### resource:<slug>
- **Name:** <canonical name>
- **Evidence:** "<verbatim quote from input>"
- **Confidence:** <high | medium | low>
- **Vault match hint:** resource:<slug>

## Concepts

- `#<tag-slug>` — "<verbatim phrase from input>"

## Notes

- <ambiguities, ownership inferences, anti-pattern flags, anything the downstream system should know>
- <if known_projects/known_areas was provided, note any borderline matches the user should confirm>
```

**Note on `Status`:** the `Status: existing | new` line appears on Projects and Areas **only** when the caller provided `known_projects` / `known_areas` lists. If no known-list was provided, omit the line entirely (everything is implicitly `new`).

### Slug convention

Slugs follow anansi's `match_key` algorithm: lowercase the canonical name, replace non-alphanumerics with spaces, collapse whitespace, join with hyphens.

- "Ian Kitajima" → `ian-kitajima`
- "PICHTR" → `pichtr`
- "O'Brien & Co." → `o-brien-co`
- "Q3 Board Deck" → `q3-board-deck`

### Empty sections

If a section has no entries, render the heading with `_(none found)_`. Downstream code can then distinguish "no projects" from "skill failed to produce that section."

---

## Confidence calibration

- **high** — Strong markers, name explicitly given, evidence is direct.
- **medium** — Markers fit but ownership ambiguous, name implicit, or category borderline.
- **low** — Name fuzzy, classification rests on inference, or document gives only a passing mention.

When confidence is medium or low, add a Note explaining what would tip the decision.

---

## Selectivity (what to extract, what to skip)

A good extractor is selective. Pull entities that are **named** (proper nouns, named projects, defined concepts), **specific enough** (not "the meeting"), and **referenced for content** (the document discusses, mentions, attributes, or relates to the entity).

Skip pronouns, indefinite references ("we", "the team"), background language ("good morning", "thanks"), and generic phrases unless given a specific name elsewhere.

**Rule:** if you can't write a sensible match_key for it, don't extract it.

---

## Deduplication

If the same entity appears multiple ways — "Ron", "Ron Nishihara", "Ron N." — emit **one** extraction with the most specific form as canonical. The evidence quote can be the strongest one.

If two entities share the same slug despite being different things, disambiguate in the canonical name and note the choice.

---

## Worked examples

### Example 1 — Email about a board meeting

**Input:**
```
Subject: Q3 Board Deck — please review

Hi James,

Thanks for finalizing the agenda for next Tuesday's DCS board meeting.
I've drafted the Q3 board deck and would love your eyes on it before
May 15. Ron Nishihara from Continest will be presenting the modular
housing pilot — I think it's worth a section on how it ties into our
sovereignty work.

Also, I read a great piece on tactical empathy in negotiation last week
that I think applies to how we're approaching the Kahoolawe meetings.

— Sarah
```

**Output:**
```
# PARA extraction: Q3 Board Deck — please review (email)

## Projects

### project:q3-board-deck
- **Name:** Q3 Board Deck
- **Evidence:** "I've drafted the Q3 board deck and would love your eyes on it before May 15"
- **Outcome:** finalize Q3 board deck
- **Deadline:** May 15
- **Confidence:** high
- **Vault match hint:** project:q3-board-deck

## Areas

### area:dcs-board-operations
- **Name:** DCS Board Operations
- **Evidence:** "next Tuesday's DCS board meeting"
- **Standard:** ongoing DCS nonprofit board governance — meetings, agenda, oversight
- **Responsibility source:** official role (user is on the board — first-person framing "our sovereignty work")
- **Confidence:** high
- **Vault match hint:** area:dcs-board-operations

### area:kahoolawe
- **Name:** Kahoolawe
- **Evidence:** "how we're approaching the Kahoolawe meetings"
- **Standard:** Kahoolawe-related civic engagement — ongoing
- **Responsibility source:** unofficial duty (user appears involved — "how we're approaching")
- **Confidence:** medium
- **Vault match hint:** area:kahoolawe

## Resources

### resource:ron-nishihara
- **Name:** Ron Nishihara
- **Evidence:** "Ron Nishihara from Continest will be presenting"
- **Confidence:** high
- **Vault match hint:** resource:ron-nishihara

### resource:continest
- **Name:** Continest
- **Evidence:** "Ron Nishihara from Continest will be presenting the modular housing pilot"
- **Confidence:** high
- **Vault match hint:** resource:continest

## Concepts

- `#modular-housing` — "the modular housing pilot"
- `#tactical-empathy` — "tactical empathy in negotiation"
- `#sovereignty` — "how it ties into our sovereignty work"

## Notes

- "DCS" classified as Area based on first-person framing ("our sovereignty work"). If user is not on the DCS board, downgrade to `resource:dcs`.
- "Kahoolawe" extracted as Area at medium confidence; could be Resource if user isn't involved in those meetings.
- "Sovereignty," "modular housing," and "tactical empathy" classified as Concepts (tags) rather than Resources — they're invoked as ideas-in-passing, not the document's headline subjects.
```

### Example 2 — YouTube transcript (no user-owned Projects/Areas)

**Input excerpt:** A YouTube transcript of a talk titled *"Why Sovereign AI Matters"* by a guest speaker. User is consuming this for reference.

**Output:**
```
# PARA extraction: "Why Sovereign AI Matters" (youtube transcript)

## Projects

_(none found)_

## Areas

_(none found)_

## Resources

### resource:reid-hoffman
- **Name:** Reid Hoffman
- **Evidence:** "as Reid Hoffman argued in his Stanford talk last year"
- **Confidence:** high
- **Vault match hint:** resource:reid-hoffman

### resource:why-sovereign-ai-matters
- **Name:** Why Sovereign AI Matters (talk)
- **Evidence:** the document itself — the topic of this transcript
- **Confidence:** medium
- **Vault match hint:** resource:why-sovereign-ai-matters

## Concepts

- `#sovereign-ai` — "the case for sovereign AI infrastructure"
- `#data-residency` — "data residency requirements at the national level"
- `#open-weights` — "open-weights models as a sovereignty enabler"

## Notes

- Transcript-shaped input. No Projects (user is consuming, not producing), no Areas (no responsibility signaled). The talk itself is a Resource (the named subject); the speakers/orgs referenced are also Resources.
- "Sovereign AI" classified as Concept tag rather than Area because the transcript signals interest, not responsibility. If the user is actively maintaining a sovereignty initiative as part of their work, it could elevate to Area in that context.
```

### Example 3 — Known-list match + the responsibility test for Areas

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

**Output:**
```
# PARA extraction: Quick update on the board deck (email)

## Projects

### project:q3-board-deck
- **Name:** Q3 Board Deck
- **Status:** existing
- **Evidence:** "The board deck is shaping up... Should be in shape by Friday."
- **Outcome:** finalize board deck (matched from known list)
- **Deadline:** Friday (consistent with known list's "active, May 15")
- **Confidence:** high
- **Vault match hint:** project:q3-board-deck

## Areas

### area:account-management-jera
- **Name:** JERA Account Management
- **Status:** existing
- **Evidence:** "On the JERA side, I had a call with their team yesterday — they want to expand the engagement scope."
- **Standard:** ongoing client relationship management — calls, scope tracking, engagement health
- **Responsibility source:** unofficial duty (user appears to be the JERA point person)
- **Confidence:** high
- **Vault match hint:** area:account-management-jera

## Resources

_(none found in this short message)_

## Concepts

- `#sovereignty` — "the sovereignty-framing slide"

## Notes

- **Hawaiian language is NOT extracted as an Area** — at least not yet. The user explicitly says "it's just a folder right now" and is *thinking about* elevating it. This is the precise inflection point Forte calls out (PARA Method, Ch. 3): a Resource becomes an Area when responsibility is taken on. Without the elevation, it's still a `resource:hawaiian-language` (or a `#hawaiian-language` concept tag in this conversation). The skill flags it for the user's awareness rather than auto-promoting.
  - **Suggested follow-up:** if the user elevates it, the next ingest of a related document will see `area:hawaiian-language` if added to the known_areas list.
- "Sovereignty" appears as a Concept tag, not a Resource. It's invoked once as a slide framing, not as the document's subject.
- Sarah is named once in passing ("Sarah's edits") — no last name; confidence is low — skipping this extraction.
```

The known-list match prevented two duplications (`project:the-board-deck` and `area:jera-stuff`) and pulled the document's casual references back to canonical vault forms. The "Hawaiian language" inflection point shows the responsibility test in action — the user is *almost* ready to elevate it, but hasn't yet, so the skill stays conservative.

---

## Edge cases

### "Is this user-owned or someone else's?"

Default to Resource (third-party) when ambiguous. Add a Note: "If user is the owner of [X], reclassify as [Project/Area]."

### "Two entities have the same slug"

Disambiguate in the canonical name. Note the disambiguation choice in Notes.

### "The document references the user themselves"

Don't extract the user as a Resource. Skip first-person references.

### "Mentioned tasks and action items"

Tasks are usually parts of a Project or Area, not standalone Projects. Skip individual action items unless they're substantial enough to warrant their own note.

### "Looks like an Area but might be a Project"

Use the deadline test. *PARA Method* (Ch. 3): "Strategic Planning," "Hiring," "Direct Reports," "Vacations" all look project-y but are areas because they're indefinite. If there's no end date, it's an Area (or a Resource if no responsibility either).

### "User is *thinking about* elevating a Resource to Area"

This is the inflection point Forte specifically discusses (the "promotion" pattern). When the document signals consideration but no commitment yet, **don't auto-promote**. Stay at Resource (or Concept tag) and Note the inflection. Let the user decide.

### "The known list says Project X is active but the document suggests it's done"

Flag in Notes. The skill's job is to surface the contradiction; the user resolves it.

### "The known list has a project the document doesn't mention"

Don't fabricate evidence. Only emit extractions for entities the document actually references.

---

## Anti-patterns to flag

### 1. Document is a topic-folder dump

A list of items grouped only by subject ("AI articles I should read"). The list items aren't PARA entities individually — they're a Resource collection. Extract the collection as one Resource, not each item.

### 2. Every mention treated as a project

If you find yourself extracting 15 Projects from one email, you're over-projectifying. *PARA Method*: a project has an outcome AND a deadline AND active commitment.

### 3. Heavy speculation on ownership

If you're inferring user ownership of an Area from very thin signals, drop confidence to low and flag.

### 4. Aspirational projects

The user mentions "I want to write a book someday" — this is a **Dream**, not a Project (per *PARA Method*'s Dreams/Projects/Hobbies test).

### 5. Promoting a topic-of-interest to Area without responsibility

If the document mentions a topic the user is collecting on but doesn't signal any responsibility, **don't elevate to Area**. Stay at Concept tag or Resource.

---

## What this skill does NOT do (out of scope)

- **Does not look up entities in the vault** beyond the known_projects/known_areas inputs. Resource lookup is downstream.
- **Does not create or modify notes.** Output is a structured extraction; routing is downstream.
- **Does not produce an anansi TOC.** Upstream of Pass 1.
- **Does not classify the input document as a whole.** Use `para-classify` for that.
- **Does not sub-type Resources** into person / organization / note — deferred for a later skill.
- **Does not handle file I/O.** User pastes content; skill processes it.
- **Does not auto-promote** Resources to Areas. The responsibility-elevation decision is the user's, not the skill's.

Future skills that build on this:

- **`resource-typer`** — given a Resource extraction, assign an anansi entity_type by matching against existing anansi entity templates. In-scope subtypes today: `entity-person.md`, `entity-organization.md`, `entity-note.md`.
- `para-route` — vault lookup → append/create routing.
- `para-link` — given an extraction and source ID, create edges.
- `anansi-toc-from-para` — render typed TOC for daemon ingestion.

---

## Process

When invoked:

1. **Parse the input.** If `known_projects:` and/or `known_areas:` blocks appear at the top (separated from the document by `---`), extract those lists.
2. **Read the document fully.**
3. **Identify candidate references.** Walk the document, flagging proper nouns, named efforts, named subjects, and tag-shaped ideas.
4. **For each candidate, decide: Concept or PARA-extractable?**
5. **For PARA candidates, walk the four-question test.** Apply the sharpened markers from *PARA Method*: Project requires outcome + deadline + active commitment; Area requires standard-to-maintain + direct responsibility.
6. **For each Project/Area, check the known list.** Mark `existing` (canonical match) or `new` (no match).
7. **Compute vault_match_hint** using the slug convention.
8. **Deduplicate.** Emit each entity once with canonical form.
9. **Emit the structured markdown.** Four sections (Projects, Areas, Resources, Concepts), then Notes.
10. **Add Notes** for ambiguities, ownership inferences, anti-pattern flags, and inflection points.

Always quote evidence verbatim. Don't paraphrase.

### Concept slug convention

Concept tags use the same slug algorithm but emit as `#<slug>`:

- "Sovereign AI" → `#sovereign-ai`
- "Tactical Empathy" → `#tactical-empathy`

These get attached as `tags: [...]` on the synergy note this document becomes.

---

## Why this matters

Downstream code reads the extraction and:

1. For **existing** Projects/Areas: append/enhance/link this source's contribution to the matched note.
2. For **new** Projects/Areas: create the note using `entity-project.md` / `entity-area.md` template, then link.
3. For Resources: route through the (future) sub-typing skill first.
4. For Concepts: append the tags to the synergy note this document becomes.
5. Write a `source_contributions` row connecting the source to each entity.

This skill is the **identification layer** of atomization. Cleaner extraction = easier every subsequent step.

The companion `para-classify` skill remains useful for one-off judgment calls. They share the same Forte-grounded decision rules; they differ in shape (one item vs. many).

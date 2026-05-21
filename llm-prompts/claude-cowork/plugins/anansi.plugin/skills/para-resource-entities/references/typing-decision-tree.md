# Typing decision tree

Walk this tree per candidate, in order. Stop at the first rule that fires.

The templates in `references/templates/` are the single source of truth for what types exist and what they require. This file specifies how to **walk** the templates against a candidate; the templates specify **what** the types are.

---

## Step 0 — Drop non-Resources (silent)

Before any typing question, check whether this candidate is a Project or Area. If so, drop it. Do not annotate the typed file. Do not flag in comments. The `para-projects-areas` skill catches it on the parallel pass.

### Project signals (drop)

A Project has all three:

- A specific outcome — verb-noun shape (`finalize Q3 deck`, `hire VP`, `ship v2`).
- A finite deadline or timeframe (`by Friday`, `before May 15`, `target Q4`).
- Active commitment, not aspirational.

Plus the user-ownership signals: first-person possessives (`our`, `my`, `we`), commitment verbs (`I'm working on`, `I need to`), explicit role context (`as DCS board chair, I…`).

### Area signals (drop)

An Area has:

- A standard to maintain — quality bar upheld indefinitely (`pay all bills on time`, `maintain fitness`).
- Direct user responsibility — official role, unofficial duty taken on, or personal commitment.

Plus the same first-person ownership signals.

### The promotion guardrail

A Resource that is central to the document — even when it is the entire topic — does **not** become an Area on that basis. Centrality is not responsibility. The actionability test is asked from the user's life, not the document's structure.

Promote into Area only when the source contains explicit ownership signals AND a standard the user is upholding. Lacking either, stay at Resource. Let `para-projects-areas` make any Area call independently.

---

## Step 1 — Person test

Does the candidate look like a named human being?

### Strong person signals

- **Name shape:** First-name + Last-name, or single given name with surrounding human-action context.
- **Honorific or title nearby:** `Dr.`, `Prof.`, `CEO`, `Senator`, `Chief`.
- **Verbs of human action attached to the name:** `said`, `wrote`, `presented`, `argued`, `replied`, `asked`, `mentioned`, `called`, `emailed`, `met with`.
- **Personal pronouns nearby** (`he`, `she`, `they`).
- **Email addresses or signatures** corresponding to the name.

### What to populate (per `entity-person.md`)

Required:

- `name` — canonical form per `references/naming-conventions.md` (full name when given; single name at lower confidence).

Optional, populate only if source provides:

- `contact_email` — verbatim from headers, signature blocks, or explicit mention.
- `contact_phone` — verbatim from signature.
- `met_via` — one short sentence on how the user came to know this person (smart-brevity style, ≤15 words).

### When to assign `person:<slug>`

- All strong signals present → `person:<slug>` at high confidence.
- Name shape clearly human but no corroborating verb or context → `person:<slug>` at medium confidence.

If name shape is ambiguous between person and organization (e.g., a single capitalized name like `Pakele`), do NOT short-circuit here. Continue to Step 2 and use the evidence verb to disambiguate.

---

## Step 2 — Organization test

Does the candidate look like a named institution?

### Strong organization signals

- **Capitalized name with no person-shape** — proper noun without a personal-name pattern.
- **Institutional framing:** `from X`, `at X`, `X announced`, `X's team`, `X's marketing department`.
- **Has employees or members** (`Sarah from Anthropic`, `the Continest team`).
- **Acronym pattern** (`PICHTR`, `DCS`, `DOE`).
- **Legal-suffix patterns** (`Inc.`, `LLC`, `PBC`, `Corp.`, `& Co.`, `Foundation`, `Department of`, `Ministry of`).
- **Domain-of-operation nearby** (`X, the climate-tech firm`).

### What to populate (per `entity-organization.md`)

Required:

- `name` — primary name as commonly used.

Optional:

- `full_name` — expanded or legal name if the source provides it.
- `type` — company / NGO / government agency / research institution / etc., when stated.
- `domain` — primary field of operation, when stated.

### When to assign `organization:<slug>`

- Strong signals present → `organization:<slug>` at high confidence.
- Capitalized proper noun + at least one institutional framing cue → high confidence.
- Capitalized proper noun, no framing, but document context makes "org" plausible → medium confidence.

### Person/organization tie-breaker

If the candidate could be either:

- Evidence verb is human-action (`said`, `wrote`) → person.
- Evidence verb is institutional (`announced`, `published`, `launched`) → organization.
- Possessive form (`Pakele's marketing department`, `Apple's iPhone`) → organization.
- Address/email format implicates a person (`@gmail.com`, signature block) → person.
- Truly unclear → pick the more probable, emit medium confidence, add a comment line. Default to `person` only when the name has clear first+last shape.

---

## Step 3 — Book test (and any other entity templates)

### Strong book signals

- **Title shape:** the source names a book by title — italicized, quoted, or capitalized in the conventional book-title way.
- **Author attribution:** `by [Author Name]`, `[Author]'s book`, `in his book`, `she writes in`.
- **Bibliographic framing:** publisher, year, ISBN, edition number, "the book argues that…".
- **Reading verbs:** `read`, `cite`, `argues in`, `writes`, `published`.

### What to populate (per `entity-book.md`)

Required:

- `title` — full title including subtitle if the source provides it. Verbatim shape (preserve em-dash or colon between title and subtitle).

Optional, populate only if source provides:

- `author` — `Last, First` format. One per line if multiple.
- `year` — four-digit year.
- `publisher` — publisher name; omit imprint parent unless relevant.
- `isbn` — 13-digit ISBN with hyphens.
- `description` — one-sentence description of the book's core claim, only if the source provides one verbatim.

### When to assign `book:<slug>`

- Title + author + bibliographic framing → `book:<slug>` at high confidence.
- Title clearly stated with author + reading verb → high confidence.
- Title clearly stated, author missing, but bibliographic framing present (`the book argues`, `cited in`) → medium confidence — still prefer `book:<slug>` over `note:<slug>` when the shape is clearly bibliographic.
- Title-only mention with no other signals → drop to `note:<slug>` with a `(book)` disambiguator only if the surrounding context truly leaves the type ambiguous; otherwise stay at `book:<slug>` medium.

**Do not** apply a `(book)` disambiguator in the canonical Name when the type is `book:` — the type itself carries the disambiguation. The `(book)` parenthetical only appears under `note:` when, for whatever reason, the candidate fell to the catchall.

### Other entity templates

For any **additional** `entity-*.md` template present in the skill's `references/templates/` (a hypothetical future `entity-event.md`, `entity-place.md`, etc.), test next in order of specificity.

For each template:

1. Read `template_class` — must be `identity`. If not, skip — `content_unit`, `source`, and `utility` templates don't belong in Resource typing.
2. Read `description` — does the candidate match the kind of thing this template represents?
3. Read `identity_fields` — does the source provide values for the required fields and most secondary fields?
4. Read `sources` hints — do the per-source-type extraction hints apply to the input shape?

If yes on description and the source can populate at least the required fields, assign `<entity_type>:<slug>`.

Only the templates currently in the skill's `references/templates/` count. The skill is template-driven — adding a template adds a candidate type without skill-code changes.

---

## Step 4 — Note fallback (catchall)

If no more-specific template fits, assign `note:<slug>` against `entity-note.md`.

### What goes here

Substantial named subjects that aren't persons, organizations, or books:

- **Talks / lectures / keynotes** — `Why Sovereign AI Matters (talk)`.
- **Podcasts / episodes** — `Acquired — TSMC (episode)`.
- **Articles / papers** — `Attention Is All You Need (paper)`.
- **Specifications / standards** — `Claude Skills Spec (spec)`.
- **Frameworks / methodologies** — `PARA Method`, `Smart Brevity`.
- **Products** — `Claude`, `iPhone 18`.
- **Places** — `Kahoolawe`.
- **Events** — `2025 Hawaii AI Summit`.
- **Named documents** — `Q3 OKR Doc`.
- **Third-party projects/areas the user does not own** — `Apple's iPhone 18 launch` is type `note` (Apple's project, the user's reference).
- **Named initiatives the user does not own**.

Apply disambiguator suffixes per `references/naming-conventions.md`.

### What to populate (per `entity-note.md`)

Required:

- `name` — canonical descriptive title-case noun phrase.

Optional:

- `summary` — one-sentence description, only if the source provides an explicit summary worth quoting verbatim.
- `content` — left blank by this skill. Discussion content lives in the source's content sections, not in the Resource block.

### Confidence on note fallback

Falling to `note` is fine and common. Confidence drops to `low` only when the candidate has no note-shaped signal — i.e., the candidate is barely substantive enough to warrant a vault entry. Default to `medium`.

---

## Step 5 — Tie-breaking

If two non-`note` templates fit equally:

1. **Prefer the more specific.** Person > organization > note. If the candidate evidence equally fits two non-note templates, take the one whose `description` is narrower.
2. **Identity-fields fit:** the template whose `identity_fields` the source can populate more completely wins. Example: if the source provides an email address, `entity-person.md`'s `contact_email` slot fills cleanly — favors `person`.
3. **Source-type hints:** read the candidate templates' `sources` hints. If one template's source-type hint matches the input shape (email vs. transcript vs. paper), it wins.

If still tied, pick the more probable, emit medium confidence, and add a comment line in the typed file noting the ambiguity.

If no template fits at all (no person, no organization, no other entity template, and the candidate doesn't even fit `note`'s catchall description), drop to `note:<slug>` with `Type confidence: low` and add a comment line: `<!-- no matching template — review needed -->`.

If the candidate's type is too unclear to commit to even `note` confidently, keep the legacy placeholder `resource:<slug>` and let downstream review resolve. Add a comment line: `<!-- type deferred — flagged for resource-typer or human review -->`.

---

## Step 6 — Confidence calibration

Three levels:

- **high** — Named explicitly with role or affiliation. Multiple corroborating signals. Identity_fields fit naturally. Re-identification on a future ingest is robust.
- **medium** — Named but unaffiliated, or partially identified. One signal contradicts. Single mention with strong context. Re-identification probable but fragile.
- **low** — Single oblique mention, no context. Fell to `note` with no note-shaped signal. Nickname-only. Re-identification is unlikely without more data.

When confidence is medium or low, add a comment line in the typed file naming what would tip the decision. Example: `<!-- medium confidence: name is single-token; surname or affiliation would raise to high -->`.

---

## Worked walkthrough

Source line: *"Just got off a call with Sarah Chen at Anthropic about their MCP rollout. She me
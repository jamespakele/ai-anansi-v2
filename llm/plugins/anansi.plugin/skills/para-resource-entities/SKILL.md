---
name: para-resource-entities
description: >
  Identify and type Resources from any input (email, transcript, meeting
  notes, article, brain dump, thread, doc) and emit two files: a typed
  Resources file with canonical names plus template assignment, and a
  Resources TOC file containing Section 3 (Discussion), Section 4
  (Resources), and a Concepts list in para-toc decimal-numbered outline
  format. Scoped solely to Resources - never extracts Projects or Areas
  and never promotes a Resource into an Area on the basis of document
  centrality. Designed to run in parallel with the para-projects-areas
  skill on the same raw input; the two outputs merge cleanly at the
  para-toc step. Template-driven typing - references/templates/ is the
  single source of truth for entity types and identity_fields. Triggers
  include "identify resources", "type resources",
  "/para-resource-entities", "extract resources from", "what
  resources are in this", "type these entities", "find the people and
  organizations in", and "build the resources TOC for".
argument-hint: "[any text - email, transcript, meeting notes, article, prose]"
---

# para-resource-entities

A focused, deterministic Resources extractor and typer. One pass over any input → two output files, both ready to merge into a typed PARA TOC.

This skill is the **Resources arm** of an atomization pipeline. It runs in parallel with `para-projects-areas` on the same raw input. The two skills do not coordinate at runtime; their outputs combine at the `para-toc` merge step.

---

## When to invoke

Trigger this skill when the user:

- Says "identify resources", "type resources", "/para-resource-entities"
- Says "extract resources from", "what resources are in this", "type these entities"
- Says "find the people and organizations in", "build the resources TOC for"
- Pastes any document and wants the people, organizations, talks, books, frameworks, places, products, events, articles, and other reference-grade entities pulled out and typed against the anansi templates
- Is running an atomization pipeline and needs the Resources lane (the Projects/Areas lane belongs to `para-projects-areas`)

Do **not** invoke for: Project or Area extraction (use `para-projects-areas`); summarizing a document (use `smart-brevity`); ingesting content as notes (use `anansi-remember`); single-item judgment (use `para-classify`).

---

## Scope — Resources only

This skill operates **after** PARA actionability sorting, in the Resource lane only. It must not touch Projects or Areas.

### What is in scope

Any named entity worth tracking that the user does not own:

- **Persons** — humans named anywhere in the source.
- **Organizations** — companies, NGOs, government bodies, agencies, teams.
- **Notes** — substantial named subjects: books, talks, podcasts, articles, frameworks, products, places, events, named documents, named initiatives the user does not own.
- **Concepts** — tag-shaped ideas referenced in passing (rendered as `#tag`, not as standalone Resources).

### What is out of scope

- **Projects** — bounded efforts the user owns. Belongs to `para-projects-areas`. Drop without comment.
- **Areas** — ongoing standards / responsibilities the user maintains. Belongs to `para-projects-areas`. Drop without comment.
- **The user themselves.** Skip first-person references.

### The promotion guardrail

A Resource that has gravity in the document — it is the topic, the headline, the central thing being discussed — is still a Resource. Centrality is not responsibility. Do not promote it into an Area on the basis of how often it appears.

The PARA actionability test is asked from the user's life, not the document's structure: *"Am I maintaining a standard for this? Am I on the hook for it?"* Unless the source contains explicit first-person ownership signals (`our`, `we`, `I'm responsible for`, `as [role], I…`), the entity stays a Resource. When in doubt, stay at Resource and let `para-projects-areas` make the call about Areas. See `references/typing-decision-tree.md`.

---

## The two output files

Both files are produced for every run. Both share the same `source_id` and `generated_at` so the downstream merge can pair them.

### File 1 — `resources-typed.md`

Fully typed Resource entities. One block per Resource, in order of first appearance in the source.

```yaml
---
source_id: <stable hash of input>
generated_at: <ISO-8601>
skill: para-resource-entities
parallel_pair: para-projects-areas
---
```

```
## Resources

### {entity_type}:{slug}
- **Name:** <canonical name per references/naming-conventions.md>
- **Template:** entity-{type}.md
- **Type confidence:** high | medium | low
- **Identity fields:** field=value; field=value (only fields the source provided)
- **Evidence:** "<verbatim quote from input>"
- **Concepts:** #tag #tag (concepts attached to or invoked alongside this Resource)
- **Vault match hint:** {entity_type}:{slug}
```

If no Resources are found, render the heading anyway with an explicit `_(no Resources found)_` line. Do not omit the heading.

### File 2 — `resources-toc.md`

Section 3 (Discussion), Section 4 (Resources), and a Concepts list, in para-toc decimal-numbered outline format.

```yaml
---
source_id: <matches resources-typed.md>
generated_at: <matches resources-typed.md>
---
```

```
## 3. Discussion
- 3.1 [discussion] <one-line section summary>
- 3.2 [discussion] <one-line section summary>

## 4. Resources
- 4.1 [{entity_type}:{slug}] <Canonical Name>
- 4.2 [{entity_type}:{slug}] <Canonical Name>

## Concepts
- `#tag` — "definition as used in source"
- `#tag` — "definition as used in source"
```

**Section 3 (Discussion)** lists the substantive content blocks of the source that anchor Resources. Discussion items are NOT separate notes — they are the canonical content sections of the source itself. Use alpha sub-addresses (`3.1.a`, `3.1.b`) only if a single discussion section needs to be split for downstream reference.

**Discussion granularity is determined by the source type's content_unit floor template.** Detect the source type, then find the `content_unit` template in `references/templates/` that (a) has the detected source type in its `sources:` block and (b) is `atomic: true`. That template is the atomization floor; its structural unit is the Discussion granularity target — one Discussion entry per instance of that unit in the source.

Example: source type `book` → `book-chapter.md` (`atomic: true`, `sources.book` present) is the floor → one Discussion entry per chapter. `book-section.md` (`atomic: false`) is a navigation container, not the floor — do not use it as the Discussion unit.

**Anti-pattern:** collapsing multiple floor-level units into a single Discussion entry. The floor template defines the finest unit — do not roll up above it.

**Section 4 (Resources)** lists each typed Resource on its own decimal line. Order matches order of first appearance in the source. Empty Section 4 is allowed but the heading must remain, with a comment line `<!-- no Resources found -->` directly below it.

**Concepts** is a flat tag list with definitions sourced from how the term is used in the document, matching `para-extract` format exactly. Concepts deviate from `para-toc`'s example by living in this skill's output rather than as a standalone Section 5; the merge step at `para-toc` carries them forward.

This skill **never** writes Section 1 or Section 2. Those belong to `para-projects-areas`.

### File format

Both files are plain markdown, UTF-8, LF line endings, terminate with a single newline. No trailing whitespace. Same input → same output.

---

## Authority order

When the rules below conflict, resolve in this order:

1. **`references/templates/`** (vendored locally; the authoritative source lives at the plugin root, but plugin skills run sandboxed and must read from their own copy) — entity types and identity_fields are defined here. The templates are the type system; this skill is a thin reader over them. Sync procedure is documented in `references/templates/README.md`.
2. **`para-extract` and `resource-typer`** — upstream skills this one partially encapsulates. Their decision rules are inherited where they apply.
3. **`para-toc`** — outline numbering and section structure. This skill produces a fragment of a `para-toc` output (Sections 3, 4, and Concepts).
4. **`resources/para-method.txt` and `resources/basb.txt`** — underlying PARA philosophy. Used when the templates and upstream skills do not resolve a question.

---

## Templates as the type system

Every file in `references/templates/` named `entity-*.md` is a candidate Resource type. Read each at invocation time — never use a hardcoded list. Adding a new entity type is a template drop-in.

Frontmatter fields the skill reads from each template:

- `entity_type` — canonical type label (`person`, `organization`, `note`, …).
- `description` — what kind of thing this represents.
- `identity_fields` — the data shape (which fields the entity tracks).
- `sources` — per-source extraction hints.

The match is reasoning-based: given the Resource's name and evidence quote, which template's combined signals fit best?

### Entity templates in scope (current vendored set)

| File | entity_type | When to pick |
|---|---|---|
| `entity-person.md` | `person` | Named individual human being. |
| `entity-organization.md` | `organization` | Named group, company, NGO, government body, team, agency. |
| `entity-book.md` | `book` | A standalone published book referenced by title (and ideally author). |
| `entity-note.md` | `note` | Catchall — substantial named subject that is not a person, organization, or book. |
| `entity-area.md` | `area` | Skip in this skill. Areas are owned by `para-projects-areas`. |
| `entity-project.md` | `project` | Skip in this skill. Projects are owned by `para-projects-areas`. |

This skill reads templates from two classes in `references/templates/`:

- **`identity` templates** (`entity-*.md`) — used for Resource typing. These define the candidate entity types (person, organization, book, note, area, project).
- **`content_unit` templates** — used for Discussion section granularity only, not for typing Resources. The `atomic: true` content_unit template whose `sources:` block includes the detected source type defines the floor — the structural unit Discussion entries map to.

`source` and `utility` templates are present in `references/templates/` but are not used by this skill. See `references/templates/README.md` for the full four-class breakdown.

`entity-area.md` and `entity-project.md` are listed for completeness — this skill never assigns them. Third-party projects and areas the user merely references belong in `note`.

If a future entity template (e.g., a hypothetical `entity-event.md`) is dropped into the authoritative source and synced here, the skill picks it up automatically as a candidate type. Specificity ordering: `person`, `organization`, `book`, any other entity types, `note` last as the catchall.

---

## Decision rules

Apply per candidate, in order. Stop at the first rule that fires.

### Step 0 — Drop non-Resources

If the candidate is a Project or an Area (per `para-extract`'s decision rules — outcome+deadline+commitment for Project; standard+responsibility for Area), drop it. Do not annotate. The parallel skill catches it.

User-ownership signals to watch for and drop on: first-person possessives (`our`, `my`, `we`, `us`, `I`), commitment verbs (`I need to`, `we're working on`, `I'm responsible for`), role context (`as DCS board chair, I…`).

### Step 1 — Person test

Does the candidate look like a named human?

- First-name + last-name shape, titles (`Dr.`, `Prof.`, `CEO`).
- Verbs of human action attached to the name: `said`, `wrote`, `presented`, `argued`, `replied`, `asked`.
- Personal pronouns nearby.

If yes → assign `person:<slug>`. Match against `entity-person.md`. Identity fields to populate when present in source: `name` (always), `contact_email`, `contact_phone`, `met_via`. Leave fields blank rather than guess.

### Step 2 — Organization test

Does the candidate look like a named institution?

- Capitalized name with no person-shape.
- Framed as an institutional affiliation: `from X`, `at X`, `X announced`, `X's team`.
- Has employees / members / a domain.

If yes → assign `organization:<slug>`. Match against `entity-organization.md`. Identity fields: `name` (always), `full_name`, `type`, `domain`. Source the org `type` from the document if stated; otherwise leave blank.

### Step 3 — Book test (and any other entity templates)

Does the candidate look like a standalone published book referenced by title?

- **Title-and-author shape:** the source mentions the title and ideally the author (`Building a Second Brain by Tiago Forte`).
- **Bibliographic framing:** publisher, year, ISBN, edition, "in his book," "the book argues," "she writes in [Title]."
- **Verbs of book-shaped reference:** `read`, `cite`, `argues in`, `writes`, `published`.

If yes → assign `book:<slug>`. Match against `entity-book.md`. Identity fields: `title` (always), `author` (when stated), `year`, `publisher`, `isbn`, `description`. Leave fields blank rather than guess. **Do not** apply a `(book)` disambiguator — the type itself is `book`, so the disambiguator becomes redundant.

If the candidate looks book-shaped but no author is stated, still prefer `book:<slug>` over `note:<slug>` when title-shape and bibliographic framing are clear; emit medium confidence.

For any **additional** entity template present in this skill's `references/templates/` (a hypothetical future `entity-event.md`, etc.), test next in order of specificity using the template's `description` and `identity_fields` to judge fit.

### Step 4 — Note fallback

If no more-specific template fits, assign `note:<slug>` against `entity-note.md`. This is the declared catchall.

For named subjects that benefit from a disambiguator (`(talk)`, `(podcast)`, `(article)`, `(spec)`, `(paper)`), apply it in the canonical name per `references/naming-conventions.md`. The slug stays based on the name without the parenthetical. Note: `(book)` is **not** used here — books take the dedicated `book:` type at Step 3.

### Step 5 — Tie-breaking

If two non-`note` templates fit equally, prefer the more specific. If still tied, pick the one whose `identity_fields` best fit the evidence (e.g., the source provides an email → `person`'s `contact_email` slot fills cleanly).

If no template fits at all, fall to `note:<slug>` with `Type confidence: low` and add a comment to the typed file noting "no matching template — review needed."

### Step 6 — Confidence calibration

- **high** — Named explicitly with role or affiliation; multiple corroborating signals; identity_fields fit naturally.
- **medium** — Named but unaffiliated, or partially identified, or one signal contradicts.
- **low** — Single oblique mention, no surrounding context, or fell to `note` with no note-shaped signal.

---

## Naming conventions

See `references/naming-conventions.md` for the full ruleset. Headline rules:

- **People** — full canonical name (`James K. Pakele`, not `James`). If only one name is given, use it as-is at lower confidence.
- **Organizations** — official name without legal suffix unless disambiguating (`Anthropic`, not `Anthropic, PBC`).
- **Notes** — descriptive title-case noun phrase. Books, talks, podcasts, articles use disambiguator suffixes: `Why Sovereign AI Matters (talk)`, `Building a Second Brain (book)`.
- **Slugs** — anansi `match_key`: lowercase, non-alphanumerics → spaces, collapse whitespace, join with hyphens. Disambiguator parentheticals are dropped from the slug.

---

## Vault match hint

Format: `{entity_type}:{kebab-case-slug}`. This is the lookup key the downstream daemon uses to upsert into the existing vault.

If the type cannot be confidently assigned at all (Step 5 fallthrough with no fit), keep the legacy `resource:<slug>` placeholder and let `resource-typer` or human review resolve later. Note the deferral in a comment line.

---

## Process

When invoked:

1. **Read the input.** Treat the entire input as the source — email, transcript, meeting notes, article, brain dump, thread, doc are all valid shapes.
2. **Compute `source_id`** as a stable hash of the trimmed input text. Compute `generated_at` as the current ISO-8601 timestamp. Both files share these values.
3. **Load templates.** Read two sets from `references/templates/`:
   - Every `entity-*.md` file (`template_class: identity`) — build the candidate Resource type table, most-specific first, `note` last.
   - Every template whose frontmatter has `template_class: content_unit` — index by `sources:` keys and `atomic` field. These define Discussion granularity in step 6, not Resource types.
4. **Walk the document.** For each candidate reference, run Step 0 first (drop Projects and Areas — silently). For each surviving candidate, walk Steps 1 → 5 to assign a type.
5. **Deduplicate.** Same physical entity referenced multiple ways → one Resource entry. Two different entities sharing a slug → disambiguate in the canonical name.
6. **Identify Discussion sections.** Detect the source type (book, meeting, newsletter, email_thread, etc.) from structural signals. Using the content_unit templates loaded in step 3, find the one that (a) lists the detected source type in its `sources:` block and (b) is `atomic: true` — this is the floor template. Map one Discussion entry to each instance of that floor unit present in the source. If no content_unit template matches the detected source type, fall back to logical content blocks. Grouping multiple floor-level units into a single Discussion entry is an error.
7. **Identify Concepts.** Tag-shaped ideas referenced in passing — extract per `para-extract`'s Concept rules. Concepts ride alongside Resources; they are not Resources themselves.
8. **Emit `resources-typed.md`.** One block per Resource in order of first appearance. Frontmatter on top.
9. **Emit `resources-toc.md`.** Section 3, Section 4, Concepts. Same `source_id` and `generated_at`.
10. **Mark inferred fields** with a brief parenthetical so reviewers can distinguish source-provided from skill-inferred. Example: `Identity fields: name=Sarah Chen; organization=Anthropic` vs. `Identity fields: name=Sarah Chen; organization=Anthropic (inferred)`.

Always quote evidence verbatim. Don't paraphrase.

---

## Worked example

See `examples/example-01-email-thread/` in this skill directory:

- `input.md` — the source document
- `resources-typed.md` — typed Resources output
- `resources-toc.md` — Sections 3, 4, and Concepts

The example is the email thread from the spec — a short message naming Sarah Chen, Anthropic, and the Claude Skills spec — and shows the deterministic mapping: one person, one organization, one note, plus two Discussion lines and two concept tags.

---

## Edge cases

### Resource is ambiguous between person and organization

Use the evidence verb to disambiguate. `Pakele said…` → person. `Pakele's marketing department…` → organization. Truly unclear → pick the more probable, emit medium confidence, add a comment line.

### Resource is a third-party project or area

`Apple's iPhone 18 launch` is Apple's project, but to the user it's a Resource. Type as `note` — anansi has no third-party-project template. Don't try to type it as `project`; that is reserved for user-owned efforts and belongs to `para-projects-areas`.

### Resource is a referenced document

`the Q3 OKR doc`, `yesterday's all-hands transcript`, `the Reuters article` — type as `note`. Anansi has separate source-type templates (`meeting-summary.md`, `email-thread.md`) but those are for documents being **ingested**, not documents being **referenced**. Referenced docs sit as `note`.

### Resource appears central to the document

Centrality is not responsibility. The talk is still a Resource (typed `note`) even when the entire input is its transcript. See Example 2 in the spec — `note:why-sovereign-ai-matters` is correct